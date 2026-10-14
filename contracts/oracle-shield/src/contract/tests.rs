#![cfg(test)]

use {
    super::*,
    soroban_sdk::testutils::{
        Address as AddressTrait, AuthorizedFunction, AuthorizedInvocation, Events, Ledger,
    },
    soroban_sdk::{Env, IntoVal, Symbol, Val, Vec, events::Event, vec},
};

fn contract_auth_for(
    env: &Env,
    address: Address,
    contract: Address,
    function: &str,
    args: impl IntoVal<Env, Vec<Val>>,
) -> (Address, AuthorizedInvocation) {
    (
        address.clone(),
        AuthorizedInvocation {
            function: AuthorizedFunction::Contract((
                contract,
                Symbol::new(env, function),
                args.into_val(env),
            )),
            sub_invocations: [].into(),
        },
    )
}

fn build_contract_client(env: &Env) -> (ContractClient<'_>, Address, Address, Address) {
    let admin_address = Address::generate(&env);
    let operator_address = Address::generate(&env);
    let max_staleness = 60_u64;
    let max_deviation: Option<u32> = None;
    let quorum: Option<u32> = None;
    let constructor_args = (
        admin_address.clone(),
        max_staleness,
        max_deviation,
        quorum,
        Some(vec![env, operator_address.clone()]),
    );
    let contract_id = env.register(Contract, constructor_args);

    (
        ContractClient::new(&env, &contract_id),
        contract_id,
        operator_address,
        admin_address,
    )
}

fn xlm_address(env: &Env) -> Address {
    Address::from_str(
        env,
        "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA",
    )
}

fn usdc_circle_address(env: &Env) -> Address {
    Address::from_str(
        env,
        "CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75",
    )
}

#[test]
fn test_constructor() {
    let env = Env::default();
    env.mock_all_auths();

    let admin_address = Address::generate(&env);
    let max_staleness = 60_u64;
    let max_deviation: Option<u32> = None;
    let quorum: Option<u32> = None;
    let operator_address = Address::generate(&env);
    let constructor_args = (
        admin_address.clone(),
        max_staleness,
        max_deviation,
        quorum,
        Some(vec![&env, operator_address]),
    );
    let contract_id = env.register(Contract, constructor_args.clone());
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            admin_address.clone(),
            contract_id.clone(),
            "__constructor",
            constructor_args
        )]
    );
}

#[test]
fn test_set_score() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, operator_address, _) = build_contract_client(&env);
    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    client.set_score(&base, &quote, &12_u32);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            operator_address.clone(),
            contract_id.clone(),
            "set_score",
            (base.clone(), quote.clone(), 12_u32,)
        )]
    );

    assert_eq!(client.get_score(&base, &quote), 12);

    let ret = client.try_set_score(&base, &quote, &12345678_u32);
    assert_eq!(ret, Err(Ok(Error::ScoreBounds)));
}

#[test]
fn test_set_score_from() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, operator_address, _) = build_contract_client(&env);
    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    client.set_score_from(&operator_address, &base, &quote, &12_u32);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            operator_address.clone(),
            contract_id.clone(),
            "set_score_from",
            (
                operator_address.clone(),
                base.clone(),
                quote.clone(),
                12_u32,
            )
        )]
    );

    assert_eq!(client.get_score(&base, &quote), 12);

    let ret = client.try_set_score_from(&operator_address, &base, &quote, &12345678_u32);
    assert_eq!(ret, Err(Ok(Error::ScoreBounds)));
}

#[test]
fn test_set_score_from_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let operator_address = Address::generate(&env);
    let (client, _, _, _) = build_contract_client(&env);
    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    let ret = client.try_set_score_from(&operator_address, &base, &quote, &12_u32);
    assert_eq!(ret, Err(Ok(Error::UnauthorizedOperator)));
}

#[test]
fn test_add_operator() {
    let env = Env::default();
    env.mock_all_auths();

    let operator_address = Address::generate(&env);
    let (client, contract_id, _, admin_address) = build_contract_client(&env);
    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    let ret = client.try_set_score_from(&operator_address, &base, &quote, &12_u32);
    assert_eq!(ret, Err(Ok(Error::UnauthorizedOperator)));

    client.add_operator(&operator_address);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            admin_address.clone(),
            contract_id.clone(),
            "add_operator",
            (operator_address.clone(),)
        )]
    );

    let ret = client.try_set_score_from(&operator_address, &base, &quote, &13_u32);
    assert_eq!(ret, Ok(Ok(())));
}

#[test]
fn test_remove_operator() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, operator_address, admin_address) = build_contract_client(&env);
    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    client.add_operator(&operator_address);

    let ret = client.try_set_score_from(&operator_address, &base, &quote, &13_u32);
    assert_eq!(ret, Ok(Ok(())));

    client.remove_operator(&operator_address);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            admin_address.clone(),
            contract_id.clone(),
            "remove_operator",
            (operator_address.clone(),)
        )]
    );

    let ret = client.try_set_score_from(&operator_address, &base, &quote, &12_u32);
    assert_eq!(ret, Err(Ok(Error::UnauthorizedOperator)));
}

#[test]
fn test_set_operators() {
    let env = Env::default();
    env.mock_all_auths();

    let operator2_address = Address::generate(&env);
    let operator3_address = Address::generate(&env);
    let (client, contract_id, operator1_address, admin_address) = build_contract_client(&env);
    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    client.add_operator(&operator1_address);

    let ret = client.try_set_score_from(&operator1_address, &base, &quote, &12_u32);
    assert_eq!(ret, Ok(Ok(())));
    let ret = client.try_set_score_from(&operator2_address, &base, &quote, &13_u32);
    assert_eq!(ret, Err(Ok(Error::UnauthorizedOperator)));
    let ret = client.try_set_score_from(&operator3_address, &base, &quote, &13_u32);
    assert_eq!(ret, Err(Ok(Error::UnauthorizedOperator)));

    let operators = vec![
        &env,
        operator2_address.clone(),
        operator2_address.clone(),
        operator3_address.clone(),
    ];
    client.set_operators(&operators);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            admin_address.clone(),
            contract_id.clone(),
            "set_operators",
            (operators,)
        )]
    );

    let ret = client.try_set_score_from(&operator1_address, &base, &quote, &12_u32);
    assert_eq!(ret, Err(Ok(Error::UnauthorizedOperator)));
    let ret = client.try_set_score_from(&operator2_address, &base, &quote, &13_u32);
    assert_eq!(ret, Ok(Ok(())));
    let ret = client.try_set_score_from(&operator3_address, &base, &quote, &13_u32);
    assert_eq!(ret, Ok(Ok(())));
}

#[test]
fn test_set_operator() {
    let env = Env::default();
    env.mock_all_auths();

    let operator_address = Address::generate(&env);
    let (client, contract_id, _, admin_address) = build_contract_client(&env);
    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    assert!(
        client
            .try_set_score_from(&operator_address, &base, &quote, &12_u32)
            .is_err()
    );

    assert_eq!(env.auths(), []);

    client.set_operator_key(&operator_address);

    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            admin_address.clone(),
            contract_id.clone(),
            "set_operator_key",
            (operator_address.clone(),)
        )]
    );
    assert!(
        client
            .try_set_score_from(&operator_address, &base, &quote, &12_u32)
            .is_ok()
    );

    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            operator_address.clone(),
            contract_id.clone(),
            "set_score_from",
            (&operator_address, &base, &quote, &12_u32,)
        )]
    );
}

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

    client.set_score_from(&operator_address, &base, &quote, &12_u32);

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

    client.set_score_from(&operator_address, &base, &quote, &0_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Unsafe);

    client.set_score_from(&operator_address, &base, &quote, &32_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Unsafe);

    client.set_score_from(&operator_address, &base, &quote, &33_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Degraded);

    client.set_score(&base, &quote, &65_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Degraded);

    client.set_score_from(&operator_address, &base, &quote, &66_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Healthy);

    client.set_score_from(&operator_address, &base, &quote, &100_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Healthy);

    assert_eq!(
        client.try_get_status(&quote, &quote),
        Err(Ok(Error::PairNotCovered))
    );
}

#[test]
fn test_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, operator_address, _) = build_contract_client(&env);
    let contract_id = client.address.clone();

    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    client.set_score_from(&operator_address, &base, &quote, &0_u32);
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

    client.set_score_from(&operator_address, &base, &quote, &10_u32);
    let event = env.events().all().filter_by_contract(&contract_id);
    let score_change = ScoreChange {
        base: base.clone(),
        quote: quote.clone(),
        score: 10,
    };
    let expected = [score_change.to_xdr(&env, &contract_id)];
    assert_eq!(event.events(), &expected);

    client.set_score_from(&operator_address, &base, &quote, &10_u32);
    assert!(
        env.events()
            .all()
            .filter_by_contract(&contract_id)
            .events()
            .is_empty()
    );

    client.set_score_from(&operator_address, &base, &quote, &44_u32);
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

    client.set_score_from(&operator_address, &base, &quote, &66_u32);
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
    let contract_id = client.address.clone();

    let val: Option<u64> = env.as_contract(&contract_id, || {
        env.storage().instance().get(&DataKey::MaxStaleness)
    });
    assert_eq!(val, Some(60_u64));

    client.set_max_staleness(&300_u64);
    let val: Option<u64> = env.as_contract(&contract_id, || {
        env.storage().instance().get(&DataKey::MaxStaleness)
    });
    assert_eq!(val, Some(300_u64));

    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    client.set_score_from(&operator_address, &base, &quote, &12_u32);
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
    client.set_score_from(&operator_address, &base, &quote, &12_u32);
    assert_eq!(client.get_score(&base, &quote), 12);
}

mod quorum;
