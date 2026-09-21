use super::*;

pub(super) fn valid_optional_date(value: Option<&str>) -> bool {
    value.is_none_or(valid_iso_date)
}

pub(super) fn valid_optional_decimal(value: Option<&str>) -> bool {
    value.is_none_or(valid_exact_decimal)
}

pub(super) fn valid_optional_non_negative_decimal(value: Option<&str>) -> bool {
    value.is_none_or(|value| !value.starts_with('-') && valid_exact_decimal(value))
}

pub(super) fn valid_iso_date(value: &str) -> bool {
    parse_iso_date(value).is_some()
}

pub(super) fn parse_iso_date(value: &str) -> Option<(i32, u32, u32)> {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year = value[0..4].parse::<i32>().ok();
    let month = value[5..7].parse::<u32>().ok();
    let day = value[8..10].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day)) = (year, month, day) else {
        return None;
    };
    let days_in_month = days_in_month(year, month)?;
    if !(1..=days_in_month).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

pub(super) fn days_in_month(year: i32, month: u32) -> Option<u32> {
    Some(match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => return None,
    })
}

pub(super) fn valid_exact_decimal(value: &str) -> bool {
    let value = value.strip_prefix('-').unwrap_or(value);
    let mut pieces = value.split('.');
    let Some(integer) = pieces.next() else {
        return false;
    };
    let fraction = pieces.next();
    if pieces.next().is_some()
        || integer.is_empty()
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || (integer.len() > 1 && integer.starts_with('0'))
    {
        return false;
    }
    fraction.is_none_or(|fraction| {
        !fraction.is_empty() && fraction.bytes().all(|byte| byte.is_ascii_digit())
    })
}

pub(super) fn decimal_magnitude(value: &str) -> Option<String> {
    valid_exact_decimal(value).then(|| value.strip_prefix('-').unwrap_or(value).to_owned())
}

pub(super) fn inverted_exact_decimal(value: &str) -> Option<String> {
    let magnitude = decimal_magnitude(value)?;
    if magnitude.bytes().all(|byte| byte == b'0' || byte == b'.') {
        return Some(magnitude);
    }
    Some(if value.starts_with('-') {
        magnitude
    } else {
        format!("-{magnitude}")
    })
}

pub(super) fn matches_inverted_original_legs(
    original_legs: &[CoreReviewLeg],
    reversal_legs: &[CoreReviewLeg],
) -> bool {
    if original_legs.len() != 2 || reversal_legs.len() != 2 {
        return false;
    }
    let mut expected = Vec::with_capacity(2);
    for leg in original_legs {
        let Some(amount_value) = inverted_exact_decimal(&leg.amount_value) else {
            return false;
        };
        expected.push((
            leg.account_id.clone(),
            leg.instrument_id.clone(),
            leg.currency.clone(),
            amount_value,
        ));
    }
    let mut actual = reversal_legs
        .iter()
        .map(|leg| {
            (
                leg.account_id.clone(),
                leg.instrument_id.clone(),
                leg.currency.clone(),
                leg.amount_value.clone(),
            )
        })
        .collect::<Vec<_>>();
    expected.sort_unstable();
    actual.sort_unstable();
    expected == actual
}

pub(super) fn valid_core_review_event(event: &CorePreparedReviewEvent) -> bool {
    event.event_class == "posting"
        && is_review_event_type(&event.event_type)
        && valid_iso_date(&event.event_date)
        && !event.spending
        && event.source_record_ids.len() == 2
        && event.source_record_ids.iter().all(|id| !id.is_empty())
        && event.source_record_ids[0] != event.source_record_ids[1]
        && event.legs.len() == 2
        && event.legs.iter().all(|leg| {
            !leg.account_id.is_empty()
                && !leg.instrument_id.is_empty()
                && !leg.currency.is_empty()
                && valid_exact_decimal(&leg.amount_value)
        })
}

/// The amount relation `0005-review-and-commit-policy.md` requires of a
/// committed relationship, re-derived from the two legs themselves.
///
/// An exact decimal as its sign and its magnitude with trailing zeros removed,
/// so `750.0` and `750.00` are one magnitude without a float ever seeing the
/// value. `None` means the value is not an exact decimal at all.
fn signed_magnitude(value: &str) -> Option<(i8, String)> {
    let magnitude = decimal_magnitude(value)?;
    let normalized = match magnitude.split_once('.') {
        // An integer magnitude keeps its zeros: `100` is not `1`.
        None => magnitude,
        Some((integer, fraction)) => match fraction.trim_end_matches('0') {
            "" => integer.to_owned(),
            fraction => format!("{integer}.{fraction}"),
        },
    };
    let sign = if normalized.bytes().all(|byte| byte == b'0' || byte == b'.') {
        0
    } else if value.starts_with('-') {
        -1
    } else {
        1
    };
    Some((sign, normalized))
}

/// Whether the two legs are a real, balanced move of the same amount.
///
/// `valid_core_review_event` only checks shape and `event_matches_group` only
/// proves each leg echoes its staged record, so neither says anything about the
/// relation *between* the legs. That relation is what stops an unbalanced event
/// from becoming an immutable ledger entry: the core checked it in
/// `packages/core/src/review-events.ts`, and the store re-checks it here
/// because the package is a proposal source, not the authority for money the
/// store writes.
pub(super) fn review_legs_balance(event: &CorePreparedReviewEvent) -> bool {
    let [first, second] = event.legs.as_slice() else {
        return false;
    };
    let (Some((first_sign, first_magnitude)), Some((second_sign, second_magnitude))) = (
        signed_magnitude(&first.amount_value),
        signed_magnitude(&second.amount_value),
    ) else {
        return false;
    };
    // Exact equal magnitude, and neither side may be zero: a zero pair moves no
    // money and has no outgoing/incoming or cash/liability direction to report.
    if first_sign == 0 || second_sign == 0 || first_magnitude != second_magnitude {
        return false;
    }
    match event.event_type.as_str() {
        // The outgoing account decreases and the incoming account increases.
        "same_currency_transfer" => first_sign != second_sign,
        // Cash decreases and the card liability decreases.
        "credit_card_repayment" => first_sign < 0 && second_sign < 0,
        _ => false,
    }
}

pub(super) fn valid_core_review_reversal(event: &CorePreparedReversalEvent) -> bool {
    matches!(
        event.event_type.as_str(),
        "same_currency_transfer_reversal" | "credit_card_repayment_reversal"
    ) && event.event_class == "posting"
        && valid_iso_date(&event.event_date)
        && !event.spending
        && event.legs.len() == 2
        && event.legs.iter().all(|leg| {
            !leg.account_id.is_empty()
                && !leg.instrument_id.is_empty()
                && !leg.currency.is_empty()
                && valid_exact_decimal(&leg.amount_value)
        })
}

pub(super) fn is_review_event_type(event_type: &str) -> bool {
    matches!(
        event_type,
        "same_currency_transfer" | "credit_card_repayment"
    )
}

pub(super) fn new_database_id(prefix: &str) -> String {
    let mut random = [0_u8; 16];
    OsRng.fill_bytes(&mut random);
    format!("{prefix}-{}", hex_encode(&random))
}

pub(super) fn event_matches_group(
    event: &CorePreparedReviewEvent,
    group: &CommitReviewGroup,
) -> bool {
    if !same_record_ids(
        &event.source_record_ids,
        &group.records[0].id,
        &group.records[1].id,
    ) {
        return false;
    }
    group.records.iter().all(|record| {
        event.legs.iter().any(|leg| {
            leg.account_id == record.account_id
                && leg.instrument_id == record.instrument_id
                && leg.currency == record.currency
                && leg.amount_value == record.account_balance_delta
        })
    })
}

pub(super) fn review_commit_key(
    event_type: &str,
    records: &[(String, String, i64, String)],
) -> String {
    let mut proposal_versions = records
        .iter()
        .map(|(_, stable_record_key, version, _)| format!("{stable_record_key}:{version}"))
        .collect::<Vec<_>>();
    proposal_versions.sort();
    let digest = Sha256::digest(
        format!(
            "{REVIEW_POLICY_VERSION}:{event_type}:{}",
            proposal_versions.join("|")
        )
        .as_bytes(),
    );
    format!("review-{}", hex_encode(&digest))
}

pub(super) fn validate_structured_parse_input(
    document_id: &str,
    input: &ValidatedStructuredParseInput,
) -> StoreResult<()> {
    if document_id.is_empty()
        || input.normalization_profile_id.is_empty()
        || input.normalization_profile_id.len() > 256
        || input.profile_json.len() > MAX_PERSISTED_PARSE_JSON_BYTES
        || input.records.is_empty()
        || input.records.len() > MAX_STRUCTURED_PARSE_RECORDS
        || !matches!(
            serde_json::from_str::<Value>(&input.profile_json),
            Ok(Value::Object(_))
        )
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid structured parse").into());
    }
    let mut stable_keys = HashSet::new();
    for record in &input.records {
        let valid_validation = matches!(
            serde_json::from_str::<Value>(&record.validation_json),
            Ok(Value::Object(values))
                if values.get("schemaValid") == Some(&Value::Bool(true))
                    && values.get("rawGrounded") == Some(&Value::Bool(true))
                    && values.get("deterministicValidationPassed") == Some(&Value::Bool(true))
        );
        if record.account_id.is_empty()
            || record.stable_record_key.is_empty()
            || record.stable_record_key.len() > 256
            || !stable_keys.insert(record.stable_record_key.as_str())
            || !matches!(
                record.record_type.as_str(),
                "transaction" | "balance" | "position" | "trade" | "valuation" | "fee" | "interest"
            )
            || record
                .event_type
                .as_deref()
                .is_some_and(|value| value.is_empty() || value.len() > 128)
            || record
                .posted_on
                .as_deref()
                .is_some_and(|value| !valid_iso_date(value))
            || record
                .posting_status
                .as_deref()
                .is_some_and(|value| !matches!(value, "provisional" | "posted"))
            || record
                .amount_value
                .as_deref()
                .is_some_and(|value| value.starts_with('-') || !valid_exact_decimal(value))
            || record
                .account_balance_delta
                .as_deref()
                .is_some_and(|value| !valid_exact_decimal(value))
            || record
                .currency
                .as_deref()
                .is_some_and(|value| !valid_currency(value))
            || record.raw_json.len() > MAX_PERSISTED_PARSE_JSON_BYTES
            || record.validation_json.len() > MAX_PERSISTED_PARSE_JSON_BYTES
            || !matches!(
                serde_json::from_str::<Value>(&record.raw_json),
                Ok(Value::Object(_))
            )
            || !valid_validation
        {
            return Err(
                io::Error::new(io::ErrorKind::InvalidInput, "invalid structured record").into(),
            );
        }
    }
    Ok(())
}

pub(super) fn valid_currency(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}
