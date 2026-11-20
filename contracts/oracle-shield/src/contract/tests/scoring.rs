use super::*;

#[test]
fn test_set_score() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, operator_address, _) = build_contract_client(&env);
    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    client.set_score(&operator_address, &base, &quote, &12_u32);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            operator_address.clone(),
            contract_id.clone(),
            "set_score",
            (
                operator_address.clone(),
                base.clone(),
                quote.clone(),
                12_u32,
            )
        )]
    );

    assert_eq!(client.get_score(&base, &quote), 12);

    let ret = client.try_set_score(&operator_address, &base, &quote, &12345678_u32);
    assert_eq!(ret, Err(Ok(Error::ScoreBounds)));
}

#[test]
fn test_set_score_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let operator_address = Address::generate(&env);
    let (client, _, _, _) = build_contract_client(&env);
    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    let ret = client.try_set_score(&operator_address, &base, &quote, &12_u32);
    assert_eq!(ret, Err(Ok(Error::UnauthorizedOperator)));
}

#[test]
fn test_set_scores() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, operator_address, _) = build_contract_client(&env);
    let base = usdc_circle_address(&env);
    let quote1 = xlm_address(&env);
    let quote2 = Address::generate(&env);
    let scores = vec![
        &env,
        (base.clone(), quote1.clone(), 12_u32),
        (base.clone(), quote2.clone(), 80_u32),
    ];

    client.set_scores(&operator_address, &scores);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            operator_address.clone(),
            contract_id.clone(),
            "set_scores",
            (operator_address.clone(), scores.clone())
        )]
    );

    assert_eq!(client.get_score(&base, &quote1), 12);
    assert_eq!(client.get_score(&base, &quote2), 80);
}

#[test]
fn test_set_scores_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let operator_address = Address::generate(&env);
    let (client, _, _, _) = build_contract_client(&env);
    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);
    let scores = vec![&env, (base, quote, 12_u32)];

    let ret = client.try_set_scores(&operator_address, &scores);
    assert_eq!(ret, Err(Ok(Error::UnauthorizedOperator)));
}

#[test]
fn test_set_scores_atomic() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, operator_address, _) = build_contract_client(&env);
    let base = usdc_circle_address(&env);
    let quote1 = xlm_address(&env);
    let quote2 = Address::generate(&env);

    client.set_score(&operator_address, &base, &quote1, &12_u32);

    // an invalid score anywhere in the batch rejects the whole batch
    let scores = vec![
        &env,
        (base.clone(), quote1.clone(), 20_u32),
        (base.clone(), quote2.clone(), 101_u32),
    ];
    let ret = client.try_set_scores(&operator_address, &scores);
    assert_eq!(ret, Err(Ok(Error::ScoreBounds)));

    assert_eq!(client.get_score(&base, &quote1), 12);
    assert!(client.try_get_score(&base, &quote2).is_err());
}

#[test]
fn test_score_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, operator_address, _) = build_contract_client(&env);
    let contract_id = client.address.clone();

    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    client.set_score(&operator_address, &base, &quote, &0_u32);
    let event = env.events().all().filter_by_contract(&contract_id);
    let score_change = ScoreChange {
        base: base.clone(),
        quote: quote.clone(),
        score: 0,
    };
    let status_change = StatusChange {
        base: base.clone(),
        quote: quote.clone(),
        status: Status::Unsafe,
    };

    let expected = [
        score_change.to_xdr(&env, &contract_id),
        status_change.to_xdr(&env, &contract_id),
    ];
    assert_eq!(event.events(), &expected);

    client.set_score(&operator_address, &base, &quote, &10_u32);
    let event = env.events().all().filter_by_contract(&contract_id);
    let score_change = ScoreChange {
        base: base.clone(),
        quote: quote.clone(),
        score: 10,
    };
    let expected = [score_change.to_xdr(&env, &contract_id)];
    assert_eq!(event.events(), &expected);

    client.set_score(&operator_address, &base, &quote, &10_u32);
    assert!(
        env.events()
            .all()
            .filter_by_contract(&contract_id)
            .events()
            .is_empty()
    );

    client.set_score(&operator_address, &base, &quote, &44_u32);
    let event = env.events().all().filter_by_contract(&contract_id);
    let score_change = ScoreChange {
        base: base.clone(),
        quote: quote.clone(),
        score: 44,
    };
    let status_change = StatusChange {
        base: base.clone(),
        quote: quote.clone(),
        status: Status::Degraded,
    };

    let expected = [
        score_change.to_xdr(&env, &contract_id),
        status_change.to_xdr(&env, &contract_id),
    ];
    assert_eq!(event.events(), &expected);

    client.set_score(&operator_address, &base, &quote, &66_u32);
    let events = env.events().all();
    let score_change_event = (
        contract_id.clone(),
        (
            Symbol::new(&env, "score_change"),
            base.clone(),
            quote.clone(),
        )
            .into_val(&env),
        66_u32.into_val(&env),
    );
    let status_change_event = (
        contract_id,
        (Symbol::new(&env, "status_change"), base, quote).into_val(&env),
        Status::Healthy.into_val(&env),
    );
    let expected = vec![&env, score_change_event, status_change_event];

    assert_eq!(events, expected)
}

#[test]
fn test_max_staleness() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, operator_address, _) = build_contract_client(&env);

    let val = client.get_parameters().max_staleness;
    assert_eq!(val, 60_u64);

    client._set_max_staleness(300_u64);
    let val = client.get_parameters().max_staleness;
    assert_eq!(val, 300_u64);

    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    client.set_score(&operator_address, &base, &quote, &12_u32);
    assert_eq!(client.get_score(&base, &quote), 12);

    env.ledger().with_mut(|ledger| {
        ledger.timestamp += 100;
    });
    assert_eq!(client.get_score(&base, &quote), 12);

    env.ledger().with_mut(|ledger| {
        ledger.timestamp += 201;
    });
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::StaleInput))
    );

    env.ledger().with_mut(|ledger| {
        ledger.timestamp += 2;
    });
    client.set_score(&operator_address, &base, &quote, &12_u32);
    assert_eq!(client.get_score(&base, &quote), 12);
}
