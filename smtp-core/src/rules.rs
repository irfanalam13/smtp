use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

const MAX_RULES: usize = 128;
const MAX_CONDITION_DEPTH: usize = 8;
const MAX_CONDITIONS: usize = 128;
const MAX_GROUP_WIDTH: usize = 32;
const MAX_ACTIONS: usize = 32;
const MAX_MATCH_VALUE_BYTES: usize = 256;
const MAX_CONTEXT_FIELD_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MailRule {
    pub id: Uuid,
    pub direction: RuleDirection,
    pub enabled: bool,
    pub priority: i32,
    pub stop_processing: bool,
    pub condition: RuleCondition,
    pub actions: Vec<RuleAction>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuleDirection {
    Incoming,
    Outgoing,
    Both,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuleCondition {
    All(Vec<RuleCondition>),
    Any(Vec<RuleCondition>),
    Not(Box<RuleCondition>),
    Text {
        field: RuleField,
        operator: RuleMatchOperator,
        value: String,
    },
    HasAttachments(bool),
    Size {
        operator: RuleSizeOperator,
        bytes: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuleField {
    From,
    To,
    Cc,
    Subject,
    BodyPreview,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuleMatchOperator {
    Equals,
    Contains,
    StartsWith,
    EndsWith,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuleSizeOperator {
    LessThan,
    LessThanOrEqual,
    Equal,
    GreaterThanOrEqual,
    GreaterThan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuleAction {
    MoveToFolder(Uuid),
    AddLabel(Uuid),
    RemoveLabel(Uuid),
    MarkRead,
    MarkUnread,
    FlagImportant,
}

#[derive(Clone, Copy, Debug)]
pub struct RuleContext<'a> {
    pub direction: RuleDirection,
    pub from: &'a str,
    pub to: &'a str,
    pub cc: &'a str,
    pub subject: &'a str,
    pub body_preview: &'a str,
    pub has_attachments: bool,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RulePlan {
    pub matched_rule_ids: Vec<Uuid>,
    pub actions: Vec<RuleAction>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RulePlanner;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum RuleError {
    #[error("rule count exceeds the configured limit")]
    TooManyRules,
    #[error("rule contains too many conditions or exceeds maximum nesting")]
    ConditionLimitExceeded,
    #[error("rule condition group is empty or too wide")]
    InvalidConditionGroup,
    #[error("rule match value is empty or too long")]
    InvalidMatchValue,
    #[error("rule action count is outside the permitted range")]
    InvalidActionCount,
    #[error("rule IDs must be unique")]
    DuplicateRuleId,
    #[error("message projection field exceeds the evaluation limit")]
    ContextFieldTooLong,
}

impl RulePlanner {
    pub fn plan(
        &self,
        rules: &[MailRule],
        context: RuleContext<'_>,
    ) -> Result<RulePlan, RuleError> {
        if rules.len() > MAX_RULES {
            return Err(RuleError::TooManyRules);
        }
        for field in [
            context.from,
            context.to,
            context.cc,
            context.subject,
            context.body_preview,
        ] {
            if field.len() > MAX_CONTEXT_FIELD_BYTES {
                return Err(RuleError::ContextFieldTooLong);
            }
        }

        let mut seen_ids = HashSet::with_capacity(rules.len());
        for rule in rules {
            if !seen_ids.insert(rule.id) {
                return Err(RuleError::DuplicateRuleId);
            }
            if rule.actions.is_empty() || rule.actions.len() > MAX_ACTIONS {
                return Err(RuleError::InvalidActionCount);
            }
            let mut condition_count = 0;
            validate_condition(&rule.condition, 1, &mut condition_count)?;
        }

        let mut ordered = rules
            .iter()
            .filter(|rule| {
                rule.enabled
                    && (rule.direction == RuleDirection::Both
                        || rule.direction == context.direction)
            })
            .collect::<Vec<_>>();
        ordered.sort_by_key(|rule| (rule.priority, rule.id));

        let mut plan = RulePlan {
            matched_rule_ids: Vec::new(),
            actions: Vec::new(),
        };
        for rule in ordered {
            if matches_condition(&rule.condition, context) {
                plan.matched_rule_ids.push(rule.id);
                plan.actions.extend(rule.actions.iter().cloned());
                if rule.stop_processing {
                    break;
                }
            }
        }
        Ok(plan)
    }
}

fn validate_condition(
    condition: &RuleCondition,
    depth: usize,
    count: &mut usize,
) -> Result<(), RuleError> {
    *count += 1;
    if depth > MAX_CONDITION_DEPTH || *count > MAX_CONDITIONS {
        return Err(RuleError::ConditionLimitExceeded);
    }
    match condition {
        RuleCondition::All(children) | RuleCondition::Any(children) => {
            if children.is_empty() || children.len() > MAX_GROUP_WIDTH {
                return Err(RuleError::InvalidConditionGroup);
            }
            for child in children {
                validate_condition(child, depth + 1, count)?;
            }
        }
        RuleCondition::Not(child) => validate_condition(child, depth + 1, count)?,
        RuleCondition::Text { value, .. } => {
            if value.is_empty() || value.len() > MAX_MATCH_VALUE_BYTES {
                return Err(RuleError::InvalidMatchValue);
            }
        }
        RuleCondition::HasAttachments(_) | RuleCondition::Size { .. } => {}
    }
    Ok(())
}

fn matches_condition(condition: &RuleCondition, context: RuleContext<'_>) -> bool {
    match condition {
        RuleCondition::All(children) => children
            .iter()
            .all(|child| matches_condition(child, context)),
        RuleCondition::Any(children) => children
            .iter()
            .any(|child| matches_condition(child, context)),
        RuleCondition::Not(child) => !matches_condition(child, context),
        RuleCondition::Text {
            field,
            operator,
            value,
        } => {
            let actual = match field {
                RuleField::From => context.from,
                RuleField::To => context.to,
                RuleField::Cc => context.cc,
                RuleField::Subject => context.subject,
                RuleField::BodyPreview => context.body_preview,
            };
            let actual = actual.to_lowercase();
            let expected = value.to_lowercase();
            match operator {
                RuleMatchOperator::Equals => actual == expected,
                RuleMatchOperator::Contains => actual.contains(&expected),
                RuleMatchOperator::StartsWith => actual.starts_with(&expected),
                RuleMatchOperator::EndsWith => actual.ends_with(&expected),
            }
        }
        RuleCondition::HasAttachments(expected) => context.has_attachments == *expected,
        RuleCondition::Size { operator, bytes } => match operator {
            RuleSizeOperator::LessThan => context.size_bytes < *bytes,
            RuleSizeOperator::LessThanOrEqual => context.size_bytes <= *bytes,
            RuleSizeOperator::Equal => context.size_bytes == *bytes,
            RuleSizeOperator::GreaterThanOrEqual => context.size_bytes >= *bytes,
            RuleSizeOperator::GreaterThan => context.size_bytes > *bytes,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> RuleContext<'static> {
        RuleContext {
            direction: RuleDirection::Incoming,
            from: "billing@example.test",
            to: "person@example.test",
            cc: "",
            subject: "Invoice April",
            body_preview: "Your invoice is attached",
            has_attachments: true,
            size_bytes: 4096,
        }
    }

    fn rule(
        id: Uuid,
        priority: i32,
        stop_processing: bool,
        condition: RuleCondition,
        action: RuleAction,
    ) -> MailRule {
        MailRule {
            id,
            direction: RuleDirection::Incoming,
            enabled: true,
            priority,
            stop_processing,
            condition,
            actions: vec![action],
        }
    }

    #[test]
    fn planner_orders_matches_and_honors_stop_processing() {
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);
        let third = Uuid::from_u128(3);
        let rules = [
            rule(
                third,
                300,
                false,
                RuleCondition::HasAttachments(true),
                RuleAction::MarkRead,
            ),
            rule(
                second,
                200,
                true,
                RuleCondition::Text {
                    field: RuleField::Subject,
                    operator: RuleMatchOperator::Contains,
                    value: "invoice".into(),
                },
                RuleAction::AddLabel(Uuid::from_u128(20)),
            ),
            rule(
                first,
                100,
                false,
                RuleCondition::All(vec![
                    RuleCondition::Text {
                        field: RuleField::From,
                        operator: RuleMatchOperator::EndsWith,
                        value: "@example.test".into(),
                    },
                    RuleCondition::Size {
                        operator: RuleSizeOperator::LessThan,
                        bytes: 8192,
                    },
                ]),
                RuleAction::MoveToFolder(Uuid::from_u128(10)),
            ),
        ];

        let plan = RulePlanner.plan(&rules, context()).expect("valid rules");
        assert_eq!(plan.matched_rule_ids, vec![first, second]);
        assert_eq!(
            plan.actions,
            vec![
                RuleAction::MoveToFolder(Uuid::from_u128(10)),
                RuleAction::AddLabel(Uuid::from_u128(20)),
            ]
        );
    }

    #[test]
    fn planner_rejects_deep_or_duplicate_rules() {
        let mut nested = RuleCondition::HasAttachments(true);
        for _ in 0..MAX_CONDITION_DEPTH {
            nested = RuleCondition::Not(Box::new(nested));
        }
        let too_deep = [rule(
            Uuid::from_u128(1),
            1,
            false,
            nested,
            RuleAction::MarkRead,
        )];
        assert_eq!(
            RulePlanner.plan(&too_deep, context()),
            Err(RuleError::ConditionLimitExceeded)
        );

        let duplicate = [
            rule(
                Uuid::from_u128(2),
                1,
                false,
                RuleCondition::HasAttachments(true),
                RuleAction::MarkRead,
            ),
            rule(
                Uuid::from_u128(2),
                2,
                false,
                RuleCondition::HasAttachments(false),
                RuleAction::MarkUnread,
            ),
        ];
        assert_eq!(
            RulePlanner.plan(&duplicate, context()),
            Err(RuleError::DuplicateRuleId)
        );
    }

    #[test]
    fn planner_rejects_oversized_context_projection() {
        let huge = "x".repeat(MAX_CONTEXT_FIELD_BYTES + 1);
        let context = RuleContext {
            subject: &huge,
            ..context()
        };
        let rules = [rule(
            Uuid::new_v4(),
            1,
            false,
            RuleCondition::HasAttachments(true),
            RuleAction::MarkRead,
        )];
        assert_eq!(
            RulePlanner.plan(&rules, context),
            Err(RuleError::ContextFieldTooLong)
        );
    }

    #[test]
    fn planner_keeps_incoming_and_outgoing_rules_separate() {
        let outgoing = MailRule {
            direction: RuleDirection::Outgoing,
            ..rule(
                Uuid::from_u128(9),
                1,
                false,
                RuleCondition::HasAttachments(true),
                RuleAction::MarkRead,
            )
        };
        let plan = RulePlanner
            .plan(&[outgoing], context())
            .expect("valid direction-scoped rule");
        assert!(plan.matched_rule_ids.is_empty());
        assert!(plan.actions.is_empty());
    }
}
