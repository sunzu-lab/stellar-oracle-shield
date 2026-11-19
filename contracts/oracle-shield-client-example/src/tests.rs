use {
    super::*,
    soroban_sdk::testutils::Address as AddressTrait,
    soroban_sdk::{Env, Map},
};

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

fn shield_contract(
    env: &Env,
) -> (
    Address,
    Address,
    stellar_oracle_shield_client::ScoringClient<'_>,
) {
    let admin_address = Address::generate(&env);
    let operator_address = Address::generate(&env);
    let max_staleness = 60_u64;
    let max_deviation: u32 = 5;
    let quorum: u32 = 1;
    let operators = Map::from_array(env, [(operator_address.clone(), ())]);
    let timelock_delay_seconds = 3_u64;
    let execution_window_seconds = 86_400_u64;
    let min_timelock_delay_seconds = 3_u64;
    let max_timelock_delay_seconds = 6_u64;
    let parameters = stellar_oracle_shield_client::Parameters {
        max_staleness,
        max_deviation,
        quorum,
        operators,
    };
    let parameters = Some(parameters);
    let governance_parameters = stellar_oracle_shield_client::GovernanceParameters {
        timelock_delay_seconds,
        execution_window_seconds,
    };
    let governance_parameters = Some(governance_parameters);
    let governance_parameters_constraints =
        stellar_oracle_shield_client::GovernanceParametersConstraints {
            min_timelock_delay_seconds,
            max_timelock_delay_seconds,
        };
    let constructor_args = (
        admin_address.clone(),
        parameters,
        governance_parameters,
        governance_parameters_constraints,
    );
    let contract_id = env.register(stellar_oracle_shield::Contract, constructor_args);

    (
        contract_id.clone(),
        operator_address,
        stellar_oracle_shield_client::ScoringClient::new(&env, &contract_id),
    )
}

fn proxy_contract_client(env: &Env, shield_contract_address: Address) -> ContractClient<'_> {
    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);
    let constructor_args = (&shield_contract_address, &base, &quote);
    let contract_id = env.register(Contract, constructor_args.clone());

    ContractClient::new(&env, &contract_id)
}

#[test]
fn test_get_status() {
    let env = Env::default();
    env.mock_all_auths();

    let (shield_contract_address, authorized_operator_address, shield_scoring_client) =
        shield_contract(&env);

    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    shield_scoring_client.set_score(&authorized_operator_address, &base, &quote, &33_u32);

    let proxy_client = proxy_contract_client(&env, shield_contract_address);
    assert_eq!(
        proxy_client.get_status(),
        stellar_oracle_shield_client::Status::Degraded
    );
}
