use super::*;

macro_rules! as_contract {
    ($env:expr, $id:expr, $($body:tt)*) => {{
        let body = || { $($body)* };
        ($env).as_contract(&($id), body)
    }};
}

fn setup_multi_operator_test() -> (Env, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|ledger| ledger.timestamp = 100);

    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    (env, base, quote)
}

fn build_multi_operator_contract_client(env: &Env) -> (ContractClient<'_>, Vec<Address>) {
    let (client, contract_id, _, _) = build_contract_client(&env);

    let mut operators = Vec::new(&env);
    for _ in 0..3 {
        operators.push_back(Address::generate(&env));
    }

    client.set_operators(&operators);

    (ContractClient::new(env, &contract_id), operators)
}

fn set_score(
    client: &ContractClient<'_>,
    base: &Address,
    quote: &Address,
    ops: &Vec<Address>,
    index: u32,
    score: u32,
) {
    client.set_score_from(&ops.get_unchecked(index), base, quote, &score);
}

#[test]
fn median_consensus_and_all_health_endpoints() {
    let (env, base, quote) = setup_multi_operator_test();
    let (client, ops) = build_multi_operator_contract_client(&env);

    client.set_quorum(&2);
    set_score(&client, &base, &quote, &ops, 0, 74);
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::QuorumNotReached))
    );
    assert_eq!(
        client.try_get_status(&base, &quote),
        Err(Ok(Error::QuorumNotReached))
    );

    set_score(&client, &base, &quote, &ops, 1, 70);
    set_score(&client, &base, &quote, &ops, 2, 10);
    let expected = AggregatedScore::new(70, 100, 100, 2, 3).unwrap();
    as_contract! {
        env,
        &client.address,
        assert_eq!(Contract::get_inner_score(&env, &Pair(base.clone(), quote.clone())), Ok(expected));
    }
    assert_eq!(client.try_get_score(&base, &quote), Ok(Ok(70)));
    assert_eq!(
        client.try_get_status(&base, &quote),
        Ok(Ok(Status::Healthy))
    );

    client.set_quorum(&3);
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::QuorumNotReached))
    );
    assert_eq!(
        client.try_get_status(&base, &quote),
        Err(Ok(Error::QuorumNotReached))
    );
}

#[test]
fn even_median_can_have_zero_status_consensus() {
    let (env, base, quote) = setup_multi_operator_test();
    let (client, ops) = build_multi_operator_contract_client(&env);

    set_score(&client, &base, &quote, &ops, 0, 0);
    set_score(&client, &base, &quote, &ops, 1, 100);
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::QuorumNotReached))
    );

    client.set_quorum(&0);
    let expected = AggregatedScore::new(50, 100, 100, 0, 3).unwrap();
    as_contract! {
        env,
        &client.address,
        assert_eq!(Contract::get_inner_score(&env, &Pair(base.clone(), quote.clone())), Ok(expected));
    }
    assert_eq!(client.try_get_score(&base, &quote), Ok(Ok(50)));
}

#[test]
fn even_median_rounds_down_at_status_boundary() {
    let (env, base, quote) = setup_multi_operator_test();
    let (client, ops) = build_multi_operator_contract_client(&env);

    set_score(&client, &base, &quote, &ops, 0, 32);
    set_score(&client, &base, &quote, &ops, 1, 33);
    let expected = AggregatedScore::new(32, 100, 100, 2, 3).unwrap();
    as_contract! {
        env,
        &client.address,
        assert_eq!(Contract::get_inner_score(&env, &Pair(base.clone(), quote.clone())), Ok(expected));
    }
    assert_eq!(client.try_get_status(&base, &quote), Ok(Ok(Status::Unsafe)));
}

#[test]
fn fresh_filter_and_quorum_are_recomputed_without_writes() {
    let (env, base, quote) = setup_multi_operator_test();
    let (client, ops) = build_multi_operator_contract_client(&env);

    set_score(&client, &base, &quote, &ops, 0, 10);
    env.ledger().with_mut(|ledger| ledger.timestamp = 130);
    set_score(&client, &base, &quote, &ops, 1, 70);
    set_score(&client, &base, &quote, &ops, 2, 74);
    let expected = AggregatedScore::new(70, 100, 130, 2, 3).unwrap();
    as_contract! {
        env,
        &client.address,
        assert_eq!(Contract::get_inner_score(&env, &Pair(base.clone(), quote.clone())), Ok(expected));
    }

    env.ledger().with_mut(|ledger| ledger.timestamp = 160);
    let expected = AggregatedScore::new(70, 100, 160, 2, 3).unwrap();
    as_contract! {
        env,
        &client.address,
        assert_eq!(Contract::get_inner_score(&env, &Pair(base.clone(), quote.clone())), Ok(expected));
    }

    env.ledger().with_mut(|ledger| ledger.timestamp = 161);
    let expected = AggregatedScore::new(72, 130, 161, 2, 3).unwrap();
    as_contract! {
        env,
        &client.address,
        assert_eq!(Contract::get_inner_score(&env, &Pair(base.clone(), quote.clone())), Ok(expected));
    }

    client.set_quorum(&2);
    env.ledger().with_mut(|ledger| ledger.timestamp = 180);
    set_score(&client, &base, &quote, &ops, 2, 90);
    env.ledger().with_mut(|ledger| ledger.timestamp = 191);
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::QuorumNotReached))
    );

    client.set_quorum(&1);
    let expected = AggregatedScore::new(90, 180, 191, 1, 3).unwrap();
    as_contract! {
        env,
        &client.address,
        assert_eq!(Contract::get_inner_score(&env, &Pair(base.clone(), quote.clone())), Ok(expected));
    }

    env.ledger().with_mut(|ledger| ledger.timestamp = 241);
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::StaleInput))
    );
}

#[test]
fn replacing_a_report_does_not_add_votes() {
    let (env, base, quote) = setup_multi_operator_test();
    let (client, ops) = build_multi_operator_contract_client(&env);

    set_score(&client, &base, &quote, &ops, 0, 70);
    set_score(&client, &base, &quote, &ops, 0, 90);
    client.set_quorum(&2);
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::QuorumNotReached))
    );
}

#[test]
fn operator_removal_takes_effect() {
    let (env, base, quote) = setup_multi_operator_test();
    let (client, ops) = build_multi_operator_contract_client(&env);

    client.set_quorum(&2);
    set_score(&client, &base, &quote, &ops, 0, 70);
    set_score(&client, &base, &quote, &ops, 1, 74);
    let expected = AggregatedScore::new(72, 100, 100, 2, 3).unwrap();
    as_contract! {
        env,
        &client.address,
        assert_eq!(Contract::get_inner_score(&env, &Pair(base.clone(), quote.clone())), Ok(expected));
    }

    client.remove_operator(&ops.get_unchecked(0));
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::QuorumNotReached))
    );

    client.set_quorum(&1);
    assert_eq!(client.try_get_score(&base, &quote), Ok(Ok(74)));
}

#[test]
fn quorum_default_take_effect() {
    let (env, _, _) = setup_multi_operator_test();
    let (client, _) = build_multi_operator_contract_client(&env);

    as_contract! {
        env,
        &client.address,
        env.storage().instance().remove(&DataKey::Quorum);
        assert_eq!(Contract::get_quorum(&env), 1);
    }
}

#[test]
fn staleness_configuration_and_upgrade() {
    let (env, base, quote) = setup_multi_operator_test();
    let (client, ops) = build_multi_operator_contract_client(&env);

    set_score(&client, &base, &quote, &ops, 0, 66);
    env.ledger().with_mut(|ledger| ledger.timestamp = 110);
    client.set_max_staleness(&9);
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::StaleInput))
    );

    client.set_max_staleness(&10);
    assert_eq!(client.try_get_score(&base, &quote), Ok(Ok(66)));
}

#[test]
fn validation_pair_isolation() {
    let (env, base, quote) = setup_multi_operator_test();
    let (client, ops) = build_multi_operator_contract_client(&env);

    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::PairNotCovered))
    );
    assert_eq!(
        client.try_set_score_from(&ops.get_unchecked(0), &base, &quote, &101),
        Err(Ok(Error::ScoreBounds))
    );

    assert_eq!(
        client.try_set_score_from(&ops.get_unchecked(0), &base, &quote, &66),
        Ok(Ok(()))
    );
    assert_eq!(client.try_get_score(&base, &quote), Ok(Ok(66)));
    assert_eq!(
        client.try_get_score(&quote, &base),
        Err(Ok(Error::PairNotCovered))
    );
}

#[test]
fn legacy_single_operator() {
    let (env, base, quote) = setup_multi_operator_test();
    let (client, ops) = build_multi_operator_contract_client(&env);

    let outsider = Address::generate(&env);
    assert_eq!(
        client.try_set_score_from(&outsider, &base, &quote, &70),
        Err(Ok(Error::UnauthorizedOperator))
    );
    assert_eq!(
        client.try_set_score(&base, &quote, &70),
        Err(Ok(Error::UnauthorizedOperator))
    );

    client.set_operator_key(&ops.get_unchecked(0));
    client.set_score(&base, &quote, &66);
    let expected = AggregatedScore::new(66, 100, 100, 1, 1).unwrap();
    as_contract! {
        env,
        &client.address,
        assert_eq!(Contract::get_inner_score(&env, &Pair(base.clone(), quote.clone())), Ok(expected));
    }
}

#[test]
fn future_report_is_excluded() {
    let (env, base, quote) = setup_multi_operator_test();
    let (client, ops) = build_multi_operator_contract_client(&env);

    env.ledger().with_mut(|ledger| ledger.timestamp = 101);
    set_score(&client, &base, &quote, &ops, 0, 70);
    env.ledger().with_mut(|ledger| ledger.timestamp = 100);
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::StaleInput))
    );
}
