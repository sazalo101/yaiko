//! Safe query filtering and sorting primitives.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterOperator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    Prefix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortField {
    pub field: String,
    pub descending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuerySpec {
    pub filters: Vec<Filter>,
    pub sort: Vec<SortField>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    InvalidFilter,
    UnsupportedOperator,
    InvalidSortField,
    InvalidLimit,
    InvalidParameter,
}

#[derive(Debug, Clone, Default)]
pub struct QueryBuilder {
    allowed_fields: BTreeSet<String>,
    filters: Vec<Filter>,
    sort: Vec<SortField>,
    limit: Option<usize>,
    cursor: Option<String>,
}

impl QueryBuilder {
    pub fn new(allowed_fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            allowed_fields: allowed_fields.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }
    pub fn parse(mut self, params: &BTreeMap<String, String>) -> Result<QuerySpec, QueryError> {
        for (key, value) in params {
            if key == "sort" {
                self.parse_sort(value)?;
            } else if key == "limit" {
                self.limit = Some(value.parse().map_err(|_| QueryError::InvalidLimit)?);
            } else if key == "cursor" {
                self.cursor = Some(value.clone());
            } else if let Some((field, operator)) = parse_filter_key(key) {
                self.filters.push(Filter {
                    field: self.checked_field(field)?,
                    operator,
                    value: value.clone(),
                });
            } else {
                return Err(QueryError::InvalidParameter);
            }
        }
        Ok(QuerySpec {
            filters: self.filters,
            sort: self.sort,
            limit: self.limit,
            cursor: self.cursor,
        })
    }

    fn parse_sort(&mut self, value: &str) -> Result<(), QueryError> {
        for raw in value.split(',').filter(|part| !part.trim().is_empty()) {
            let raw = raw.trim();
            let descending = raw.starts_with('-');
            let field = raw.trim_start_matches('-');
            self.sort.push(SortField {
                field: self.checked_field(field)?,
                descending,
            });
        }
        Ok(())
    }

    fn checked_field(&self, field: &str) -> Result<String, QueryError> {
        if self.allowed_fields.contains(field)
            && field
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            Ok(field.to_string())
        } else {
            Err(QueryError::InvalidSortField)
        }
    }
}

fn parse_filter_key(key: &str) -> Option<(&str, FilterOperator)> {
    let (field, operator) = key.split_once('[')?;
    let operator = operator.strip_suffix(']')?;
    let operator = match operator {
        "eq" => FilterOperator::Eq,
        "ne" => FilterOperator::Ne,
        "gt" => FilterOperator::Gt,
        "gte" => FilterOperator::Gte,
        "lt" => FilterOperator::Lt,
        "lte" => FilterOperator::Lte,
        "contains" => FilterOperator::Contains,
        "prefix" => FilterOperator::Prefix,
        _ => return None,
    };
    Some((field, operator))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_filters_sorts_limits_and_cursor() {
        let params = BTreeMap::from([
            (String::from("name[contains]"), String::from("Ada")),
            (String::from("sort"), String::from("-created_at,name")),
            (String::from("limit"), String::from("25")),
            (String::from("cursor"), String::from("opaque")),
        ]);
        let query = QueryBuilder::new(["name", "created_at"])
            .parse(&params)
            .unwrap();
        assert_eq!(query.filters[0].operator, FilterOperator::Contains);
        assert!(query.sort[0].descending);
        assert_eq!(query.limit, Some(25));
        assert_eq!(query.cursor.as_deref(), Some("opaque"));
    }

    #[test]
    fn rejects_unallowlisted_fields_and_malformed_parameters() {
        let params = BTreeMap::from([(String::from("sort"), String::from("password"))]);
        assert_eq!(
            QueryBuilder::new(["name"]).parse(&params),
            Err(QueryError::InvalidSortField)
        );
        let params = BTreeMap::from([(String::from("name[unknown]"), String::from("x"))]);
        assert_eq!(
            QueryBuilder::new(["name"]).parse(&params),
            Err(QueryError::InvalidParameter)
        );
    }

    #[test]
    fn accepts_only_safe_field_identifiers() {
        let params = BTreeMap::from([(String::from("name[eq]"), String::from("x"))]);
        assert!(QueryBuilder::new(["name"]).parse(&params).is_ok());
        let params = BTreeMap::from([(String::from("name[eq]"), String::from("x"))]);
        assert!(QueryBuilder::new(["name;drop"]).parse(&params).is_err());
    }
}
