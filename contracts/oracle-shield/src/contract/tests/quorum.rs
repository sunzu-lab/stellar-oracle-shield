use super::*;

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

    let mut operators = Map::new(&env);
    for _ in 0..3 {
        operators.set(Address::generate(&env), ());
    }

    client.schedule_set_operators(&operators);
    client._wait_timelock();
    client.execute_set_parameters();

    (ContractClient::new(env, &contract_id), operators.keys())
}

fn set_score(
    client: &ContractClient<'_>,
    base: &Address,
    quote: &Address,
    ops: &Vec<Address>,
    index: u32,
    score: u32,
) {
    client.set_score(&ops.get_unchecked(index), base, quote, &score);
}

#[test]
fn median_consensus_and_all_health_endpoints() {
    let (env, base, quote) = setup_multi_operator_test();
    let (client, ops) = build_multi_operator_contract_client(&env);

    client._set_quorum(2);
    let ts_1 = env.ledger().timestamp();
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
    let expected = AggregatedScore::new(70, ts_1, ts_1, 2, 3).unwrap();
    let aggregated_score = client._get_inner_score(&base, &quote);
    assert_eq!(aggregated_score, Ok(expected));
    assert_eq!(client.try_get_score(&base, &quote), Ok(Ok(70)));
    assert_eq!(
        client.try_get_status(&base, &quote),
        Ok(Ok(Status::Healthy))
    );

    client._set_quorum(3);
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

    let ts_1 = env.ledger().timestamp();
    set_score(&client, &base, &quote, &ops, 0, 0);
    set_score(&client, &base, &quote, &ops, 1, 100);
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::QuorumNotReached))
    );

    client._set_quorum(0);
    let ts_2 = env.ledger().timestamp();
    let expected = AggregatedScore::new(50, ts_1, ts_2, 0, 3).unwrap();
    let aggregated_score = client._get_inner_score(&base, &quote);
    assert_eq!(aggregated_score, Ok(expected));
    assert_eq!(client.try_get_score(&base, &quote), Ok(Ok(50)));
}

#[test]
fn even_median_rounds_down_at_status_boundary() {
    let (env, base, quote) = setup_multi_operator_test();
    let (client, ops) = build_multi_operator_contract_client(&env);

    set_score(&client, &base, &quote, &ops, 0, 32);
    set_score(&client, &base, &quote, &ops, 1, 33);
    let oldest_ts = env.ledger().timestamp();
    let newest_ts = env.ledger().timestamp();
    let expected = AggregatedScore::new(32, oldest_ts, newest_ts, 2, 3).unwrap();
    let aggregated_score = client._get_inner_score(&base, &quote);
    assert_eq!(aggregated_score, Ok(expected));
    assert_eq!(client.try_get_status(&base, &quote), Ok(Ok(Status::Unsafe)));
}

#[test]
fn fresh_filter_and_quorum_are_recomputed_without_writes() {
    let (env, base, quote) = setup_multi_operator_test();
    let (client, ops) = build_multi_operator_contract_client(&env);

    let ts_1 = env.ledger().timestamp();
    set_score(&client, &base, &quote, &ops, 0, 10);

    let ts_2 = ts_1 + 30;
    env.ledger().with_mut(|ledger| ledger.timestamp = ts_2);
    set_score(&client, &base, &quote, &ops, 1, 70);
    set_score(&client, &base, &quote, &ops, 2, 74);
    let expected = AggregatedScore::new(70, ts_1, ts_2, 2, 3).unwrap();
    let aggregated_score = client._get_inner_score(&base, &quote);
    assert_eq!(aggregated_score, Ok(expected));

    let ts_3 = ts_2 + 30;
    env.ledger().with_mut(|ledger| ledger.timestamp = ts_3);
    let expected = AggregatedScore::new(70, ts_1, ts_3, 2, 3).unwrap();
    let aggregated_score = client._get_inner_score(&base, &quote);
    assert_eq!(aggregated_score, Ok(expected));

    let ts_4 = ts_3 + 1;
    env.ledger().with_mut(|ledger| ledger.timestamp = ts_4);
    let expected = AggregatedScore::new(72, ts_2, ts_4, 2, 3).unwrap();
    let aggregated_score = client._get_inner_score(&base, &quote);
    assert_eq!(aggregated_score, Ok(expected));

    client._set_quorum(2);

    let ts_5 = ts_4 + 19;
    env.ledger().with_mut(|ledger| ledger.timestamp = ts_5);
    set_score(&client, &base, &quote, &ops, 2, 90);

    let ts_6 = ts_5 + 11;
    env.ledger().with_mut(|ledger| ledger.timestamp = ts_6);
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::QuorumNotReached))
    );

    client._set_quorum(1);
    let expected = AggregatedScore::new(90, ts_5, ts_6, 1, 3).unwrap();
    let aggregated_score = client._get_inner_score(&base, &quote);
    assert_eq!(aggregated_score, Ok(expected));

    let ts_7 = ts_6 + 61;
    env.ledger().with_mut(|ledger| ledger.timestamp = ts_7);
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
    client._set_quorum(2);
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::QuorumNotReached))
    );
}

#[test]
fn operator_removal_takes_effect() {
    let (env, base, quote) = setup_multi_operator_test();
    let (client, ops) = build_multi_operator_contract_client(&env);

    client._set_quorum(2);
    set_score(&client, &base, &quote, &ops, 0, 70);
    set_score(&client, &base, &quote, &ops, 1, 74);
    let ts_1 = env.ledger().timestamp();
    let expected = AggregatedScore::new(72, ts_1, ts_1, 2, 3).unwrap();
    let aggregated_score = client._get_inner_score(&base, &quote);
    assert_eq!(aggregated_score, Ok(expected));

    client._remove_operator(ops.get_unchecked(0).clone());
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::QuorumNotReached))
    );

    client._set_quorum(1);
    assert_eq!(client.try_get_score(&base, &quote), Ok(Ok(74)));
}

#[test]
fn quorum_default_take_effect() {
    let (env, _, _) = setup_multi_operator_test();
    let (client, _) = build_multi_operator_contract_client(&env);

    let body = || {
        env.storage().instance().remove(&DataKey::ParametersV2);
    };
    env.as_contract(&client.address, body);
    assert_eq!(client.get_parameters().quorum, 0);
}

#[test]
fn staleness_configuration_and_upgrade() {
    let (env, base, quote) = setup_multi_operator_test();
    let (client, ops) = build_multi_operator_contract_client(&env);

    set_score(&client, &base, &quote, &ops, 0, 66);
    env.ledger().with_mut(|ledger| ledger.timestamp += 10);
    client._set_max_staleness(9);
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::StaleInput))
    );

    client._set_max_staleness(10);
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
