use super::*;

#[test]
fn test_get_score() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, operator_address, _) = build_contract_client(&env);

    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::PairNotCovered))
    );
    assert!(env.auths().is_empty());

    client.set_score(&operator_address, &base, &quote, &12_u32);

    assert_eq!(client.get_score(&base, &quote), 12);
    assert!(env.auths().is_empty());

    assert_eq!(
        client.try_get_score(&quote, &quote),
        Err(Ok(Error::PairNotCovered))
    );
}

#[test]
fn test_get_status() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, operator_address, _) = build_contract_client(&env);

    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);
    assert_eq!(
        client.try_get_status(&base, &quote),
        Err(Ok(Error::PairNotCovered))
    );
    assert!(env.auths().is_empty());

    client.set_score(&operator_address, &base, &quote, &0_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Unsafe);

    client.set_score(&operator_address, &base, &quote, &32_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Unsafe);

    client.set_score(&operator_address, &base, &quote, &33_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Degraded);

    client.set_score(&operator_address, &base, &quote, &65_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Degraded);

    client.set_score(&operator_address, &base, &quote, &66_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Healthy);

    client.set_score(&operator_address, &base, &quote, &100_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Healthy);

    assert_eq!(
        client.try_get_status(&quote, &quote),
        Err(Ok(Error::PairNotCovered))
    );
}
