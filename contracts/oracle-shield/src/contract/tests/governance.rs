use super::*;

#[test]
fn test_add_operator() {
    let env = Env::default();
    env.mock_all_auths();

    let operator_address = Address::generate(&env);
    let (client, contract_id, _, admin_address) = build_contract_client(&env);
    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    let ret = client.try_set_score(&operator_address, &base, &quote, &12_u32);
    assert_eq!(ret, Err(Ok(Error::UnauthorizedOperator)));

    client.schedule_add_operator(&operator_address);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            admin_address.clone(),
            contract_id.clone(),
            "schedule_add_operator",
            (operator_address.clone(),)
        )]
    );

    client._wait_timelock();
    client.execute_set_parameters();

    let ret = client.try_set_score(&operator_address, &base, &quote, &13_u32);
    assert_eq!(ret, Ok(Ok(())));
}

#[test]
fn test_remove_operator() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, operator_address, admin_address) = build_contract_client(&env);
    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    let second_operator_address = Address::generate(&env);

    client.schedule_add_operator(&second_operator_address);
    client._wait_timelock();
    client.execute_set_parameters();

    let ret = client.try_set_score(&operator_address, &base, &quote, &13_u32);
    assert_eq!(ret, Ok(Ok(())));

    client.schedule_remove_operator(&operator_address);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            admin_address.clone(),
            contract_id.clone(),
            "schedule_remove_operator",
            (operator_address.clone(),)
        )]
    );
    client._wait_timelock();
    client.execute_set_parameters();

    // operator_address is not removed
    let ret = client.try_set_score(&operator_address, &base, &quote, &12_u32);
    assert_eq!(ret, Err(Ok(Error::UnauthorizedOperator)));

    // number of operators gets below quorum
    let ret = client.try_schedule_remove_operator(&second_operator_address);
    assert_eq!(ret, Err(Ok(Error::InvalidParam)));
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

    client.schedule_add_operator(&operator1_address);
    client._wait_timelock();
    client.execute_set_parameters();

    let ret = client.try_set_score(&operator1_address, &base, &quote, &12_u32);
    assert_eq!(ret, Ok(Ok(())));
    let ret = client.try_set_score(&operator2_address, &base, &quote, &13_u32);
    assert_eq!(ret, Err(Ok(Error::UnauthorizedOperator)));
    let ret = client.try_set_score(&operator3_address, &base, &quote, &13_u32);
    assert_eq!(ret, Err(Ok(Error::UnauthorizedOperator)));

    let operators = Map::from_array(
        &env,
        [
            (operator2_address.clone(), ()),
            (operator2_address.clone(), ()),
            (operator3_address.clone(), ()),
        ],
    );
    client.schedule_set_operators(&operators);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            admin_address.clone(),
            contract_id.clone(),
            "schedule_set_operators",
            (operators,)
        )]
    );
    client._wait_timelock();
    client.execute_set_parameters();

    let ret = client.try_set_score(&operator1_address, &base, &quote, &12_u32);
    assert_eq!(ret, Err(Ok(Error::UnauthorizedOperator)));
    let ret = client.try_set_score(&operator2_address, &base, &quote, &13_u32);
    assert_eq!(ret, Ok(Ok(())));
    let ret = client.try_set_score(&operator3_address, &base, &quote, &13_u32);
    assert_eq!(ret, Ok(Ok(())));
}

#[test]
fn test_parameters_change_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, _, _) = build_contract_client(&env);
    let initial_parameters = client.get_parameters();

    client.schedule_set_max_staleness(&300_u64);
    client._wait_timelock();
    client.execute_set_parameters();
    let event = env.events().all().filter_by_contract(&contract_id);
    let parameters = Parameters {
        max_staleness: 300,
        ..initial_parameters
    };
    let parameters_change = ParametersChange { parameters };
    let expected = [parameters_change.to_xdr(&env, &contract_id)];
    assert_eq!(event.events(), &expected);
}

#[test]
fn test_admin_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, _, admin_address) = build_contract_client(&env);
    let timelock_delay_seconds = client.get_governance_params().timelock_delay_seconds;

    let new_admin_address = Address::generate(&env);
    let another_operator = Address::generate(&env);
    let valid_after = env.ledger().timestamp() + timelock_delay_seconds;

    client.schedule_admin_transfer(&new_admin_address);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            admin_address.clone(),
            contract_id.clone(),
            "schedule_admin_transfer",
            (new_admin_address.clone(),)
        )]
    );
    let events = env.events().all();
    let address = new_admin_address.clone();
    let candidate = address.into_time_locked(valid_after);
    let admin_transfer_initiated_event = AdminTransferInitiated { admin: candidate };
    let expected = [admin_transfer_initiated_event.to_xdr(&env, &contract_id)];
    assert_eq!(events, expected);

    client.schedule_add_operator(&another_operator);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            admin_address.clone(), // the point
            contract_id.clone(),
            "schedule_add_operator",
            (another_operator.clone(),)
        )]
    );
    client.cancel_set_parameters();

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + timelock_delay_seconds);

    client.execute_admin_transfer();
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            new_admin_address.clone(),
            contract_id.clone(),
            "execute_admin_transfer",
            ()
        )]
    );
    let events = env.events().all();
    let new_admin = new_admin_address.clone();
    let admin_transfered_event = AdminTransfered { admin: new_admin };
    let expected = [admin_transfered_event.to_xdr(&env, &contract_id)];
    assert_eq!(events, expected);

    client.schedule_add_operator(&another_operator);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            new_admin_address.clone(), // the point
            contract_id.clone(),
            "schedule_add_operator",
            (another_operator.clone(),)
        )]
    );
}

#[test]
fn test_admin_transfer_cancels_scheduled_operations() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, _, _) = build_contract_client(&env);
    let timelock_delay_seconds = client.get_governance_params().timelock_delay_seconds;

    let new_admin_address = Address::generate(&env);
    let valid_after = env.ledger().timestamp() + timelock_delay_seconds;

    client.schedule_set_max_staleness(&1);
    client.schedule_admin_transfer(&new_admin_address);
    client.schedule_set_governance_params(&GovernanceParameters {
        timelock_delay_seconds: 3,
        execution_window_seconds: 86_400,
    });
    client.schedule_upgrade(&BytesN::from_array(&env, &[0; 32]));

    env.ledger().set_timestamp(valid_after);

    client.execute_admin_transfer();
    let topics = get_all_topics(&env, &env.events().all());
    let expected = std::vec![
        "parameters_change_cancelled".to_string(),
        "governance_params_change_cancel".to_string(),
        "upgrade_cancelled".to_string(),
        "admin_transfered".to_string()
    ];
    assert_eq!(topics, expected);

    assert_eq!(client.get_scheduled_parameters(), None);
    assert_eq!(client.get_scheduled_governance_params(), None);
    assert_eq!(client.get_scheduled_upgrade(), None);
}

#[test]
fn test_admin_transfer_cancellation() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, _, _) = build_contract_client(&env);

    let new_admin_address = Address::generate(&env);
    client.schedule_admin_transfer(&new_admin_address);

    client.cancel_admin_transfer();
    let events = env.events().all();
    let admin = new_admin_address.clone();
    let admin_transfer_cancelled_event = AdminTransferCancelled { admin };
    let expected = [admin_transfer_cancelled_event.to_xdr(&env, &contract_id)];
    assert_eq!(events, expected);

    // too late
    let ret = client.try_execute_admin_transfer();
    assert_eq!(ret, Err(Ok(Error::Expired)));
}

#[test]
fn test_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, _, _) = build_contract_client(&env);
    let timelock_delay_seconds = client.get_governance_params().timelock_delay_seconds;

    let new_admin_address = Address::generate(&env);
    let new_governance_parameters = GovernanceParameters {
        timelock_delay_seconds: 3,
        execution_window_seconds: 86_400,
    };

    client.schedule_set_max_staleness(&1);
    client.schedule_set_governance_params(&new_governance_parameters);
    client.schedule_admin_transfer(&new_admin_address);
    client.schedule_upgrade(&BytesN::from_array(&env, &[0; 32]));

    // second schedule fails
    let ret = client.try_schedule_set_max_deviation(&1);
    assert_eq!(ret, Err(Ok(Error::ScheduledOperationInProgress)));

    let ret = client.try_schedule_set_governance_params(&new_governance_parameters);
    assert_eq!(ret, Err(Ok(Error::ScheduledOperationInProgress)));

    let ret = client.try_schedule_admin_transfer(&new_admin_address);
    assert_eq!(ret, Err(Ok(Error::ScheduledOperationInProgress)));

    let ret = client.try_schedule_upgrade(&BytesN::from_array(&env, &[0; 32]));
    assert_eq!(ret, Err(Ok(Error::ScheduledOperationInProgress)));

    // cancellation
    client.cancel_set_parameters();
    client.cancel_set_governance_params();
    client.cancel_admin_transfer();
    client.cancel_upgrade();

    // execution fails with Expire
    let ret = client.try_execute_set_parameters();
    assert_eq!(ret, Err(Ok(Error::Expired)));

    let ret = client.try_execute_set_governance_params();
    assert_eq!(ret, Err(Ok(Error::Expired)));

    let ret = client.try_execute_admin_transfer();
    assert_eq!(ret, Err(Ok(Error::Expired)));

    let ret = client.try_execute_upgrade();
    assert_eq!(ret, Err(Ok(Error::Expired)));

    let valid_after = env.ledger().timestamp() + timelock_delay_seconds;

    client.schedule_set_max_staleness(&1);
    client.schedule_set_governance_params(&new_governance_parameters);
    client.schedule_admin_transfer(&new_admin_address);
    client.schedule_upgrade(&BytesN::from_array(&env, &[0; 32]));

    env.ledger().set_timestamp(valid_after - 1);

    // too soon
    let ret = client.try_execute_set_parameters();
    assert_eq!(ret, Err(Ok(Error::TimeLocked)));

    let ret = client.try_execute_set_governance_params();
    assert_eq!(ret, Err(Ok(Error::TimeLocked)));

    let ret = client.try_execute_admin_transfer();
    assert_eq!(ret, Err(Ok(Error::TimeLocked)));

    let ret = client.try_execute_upgrade();
    assert_eq!(ret, Err(Ok(Error::TimeLocked)));

    env.ledger().with_mut(|l| {
        l.sequence_number += 500 * 86400;
    });

    // too late
    let ret = client.try_execute_set_parameters();
    assert_eq!(ret, Err(Ok(Error::Expired)));

    let ret = client.try_execute_set_governance_params();
    assert_eq!(ret, Err(Ok(Error::Expired)));

    let ret = client.try_execute_admin_transfer();
    assert_eq!(ret, Err(Ok(Error::Expired)));

    let ret = client.try_execute_upgrade();
    assert_eq!(ret, Err(Ok(Error::Expired)));

    let valid_after = env.ledger().timestamp() + timelock_delay_seconds;

    client.schedule_set_max_staleness(&1);
    client.schedule_set_governance_params(&new_governance_parameters);
    client.schedule_admin_transfer(&new_admin_address);
    client.schedule_upgrade(&BytesN::from_array(&env, &[0; 32]));

    // right on time
    env.ledger().set_timestamp(valid_after);

    let ret = client.try_execute_set_parameters();
    assert_eq!(ret, Ok(Ok(())));

    let ret = client.try_execute_set_governance_params();
    assert_eq!(ret, Ok(Ok(())));

    let ret = client.try_execute_admin_transfer();
    assert_eq!(ret, Ok(Ok(())));

    let valid_after = env.ledger().timestamp() + timelock_delay_seconds;
    client.schedule_upgrade(&BytesN::from_array(&env, &[0; 32]));
    env.ledger().set_timestamp(valid_after);

    let ret = client.try_execute_upgrade();
    assert_eq!(ret, Err(Err(soroban_sdk::InvokeError::Abort))); // cannot ugrade but not Timelock nor Expired
}

#[test]
fn test_upgrade_0_2_0_to_0_3_0() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, operator_address, _) = build_contract_client(&env);

    // simulate post migration situation
    let body = || {
        let storage = env.storage().instance();
        storage.remove(&DataKey::ParametersV2);
        storage.remove(&DataKey::GovernanceParameters);
        storage.remove(&DataKey::GovernanceParametersConstraints);

        let parameters_v1 = parameters_v1::Parameters {
            max_staleness: 7,
            max_deviation: 8,
            quorum: 9,
        };

        storage.set(&DataKey::Parameters, &parameters_v1);

        let operators: Map<Address, ()> = Map::from_array(&env, [(operator_address.clone(), ())]);
        storage.set(&DataKey::Operators, &operators);
    };
    env.as_contract(&client.address, body);

    // trigger upgrade
    let parameters = client.get_parameters();
    let governance_parameters = client.get_governance_params();
    let governance_parameters_constraints = client.get_governance_constraints();

    let expected_parameters = Parameters {
        max_staleness: 7,
        max_deviation: 8,
        quorum: 9,
        operators: Map::from_array(&env, [(operator_address.clone(), ())]),
    };
    let expected_governance_parameters = GovernanceParameters::default();
    let expected_governance_parameters_constraints = GovernanceParametersConstraints::default();

    assert_eq!(parameters, expected_parameters);
    assert_eq!(governance_parameters, expected_governance_parameters);
    assert_eq!(
        governance_parameters_constraints,
        expected_governance_parameters_constraints
    );

    // check storage
    let body = || {
        let storage = env.storage().instance();
        (
            storage.get::<_, parameters_v1::Parameters>(&DataKey::Parameters),
            storage.get::<_, Map<Address, ()>>(&DataKey::Operators),
            storage.get::<_, Parameters>(&DataKey::ParametersV2),
            storage.get::<_, GovernanceParameters>(&DataKey::GovernanceParameters),
            storage.get::<_, GovernanceParametersConstraints>(
                &DataKey::GovernanceParametersConstraints,
            ),
        )
    };
    let (
        storage_parameters_v1,
        storage_operators,
        storage_parameters,
        storage_governance_parameters,
        storage_governance_parameters_constraints,
    ) = env.as_contract(&client.address, body);

    assert_eq!(storage_parameters_v1, None);
    assert_eq!(storage_operators, None);
    assert_eq!(storage_parameters, Some(expected_parameters));
    assert_eq!(
        storage_governance_parameters,
        Some(expected_governance_parameters)
    );
    assert_eq!(
        storage_governance_parameters_constraints,
        Some(expected_governance_parameters_constraints)
    );
}

#[test]
fn test_schedule_governance_parameters() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, _, admin_address) = build_contract_client(&env);

    let current_governance_parameters = client.get_governance_params();
    let governance_parameters_constraints = client.get_governance_constraints();

    let new_governance_parameters = GovernanceParameters {
        timelock_delay_seconds: governance_parameters_constraints.min_timelock_delay_seconds - 1,
        execution_window_seconds: 86_400,
    };

    let ret = client.try_schedule_set_governance_params(&new_governance_parameters);
    assert_eq!(ret, Err(Ok(Error::InvalidParam)));

    let new_governance_parameters = GovernanceParameters {
        timelock_delay_seconds: governance_parameters_constraints.max_timelock_delay_seconds + 1,
        execution_window_seconds: 86_400,
    };

    let ret = client.try_schedule_set_governance_params(&new_governance_parameters);
    assert_eq!(ret, Err(Ok(Error::InvalidParam)));

    let new_governance_parameters = GovernanceParameters {
        timelock_delay_seconds: governance_parameters_constraints.min_timelock_delay_seconds,
        execution_window_seconds: 86_400,
    };

    client.schedule_set_governance_params(&new_governance_parameters);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            admin_address.clone(),
            contract_id.clone(),
            "schedule_set_governance_params",
            (new_governance_parameters.clone(),)
        )]
    );

    let ret = client.try_execute_set_governance_params();
    assert_eq!(ret, Err(Ok(Error::TimeLocked)));

    let t0 = env.ledger().timestamp();
    let t1 = t0 + new_governance_parameters.timelock_delay_seconds;
    let t2 = t0 + current_governance_parameters.timelock_delay_seconds;

    env.ledger().set_timestamp(t1);

    // still locked after new timelock delay
    let ret = client.try_execute_set_governance_params();
    assert_eq!(ret, Err(Ok(Error::TimeLocked)));

    env.ledger().set_timestamp(t2);

    // ok after current timelock delay
    let ret = client.try_execute_set_governance_params();
    assert_eq!(ret, Ok(Ok(())));

    let events = env.events().all();
    let governance_param_changed_event = GovernanceParamsChange {
        parameters: new_governance_parameters,
    };
    let expected = [governance_param_changed_event.to_xdr(&env, &contract_id)];
    assert_eq!(events, expected);

    assert_eq!(client.get_governance_params(), new_governance_parameters);
}
