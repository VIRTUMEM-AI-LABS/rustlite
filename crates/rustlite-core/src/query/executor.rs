/// Query executor
///
/// Executes physical query plans using iterators.
use super::ast::*;
use super::planner::{PhysicalOperator, PhysicalPlan};
use crate::error::Result;
use std::collections::HashMap;
use std::fmt;

/// Query result row
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub columns: Vec<Column>,
    pub values: Vec<Value>,
}

/// Column metadata
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub name: String,
    pub alias: Option<String>,
}

/// Value types in query results
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
}

impl Value {
    /// Convert value to bytes for comparison
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Value::Integer(i) => i.to_le_bytes().to_vec(),
            Value::Float(f) => f.to_le_bytes().to_vec(),
            Value::String(s) => s.as_bytes().to_vec(),
            Value::Boolean(b) => vec![if *b { 1 } else { 0 }],
            Value::Null => vec![],
        }
    }

    /// Compare values
    pub fn compare(&self, other: &Value, op: &BinaryOperator) -> bool {
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => match op {
                BinaryOperator::Eq => a == b,
                BinaryOperator::Ne => a != b,
                BinaryOperator::Lt => a < b,
                BinaryOperator::Le => a <= b,
                BinaryOperator::Gt => a > b,
                BinaryOperator::Ge => a >= b,
            },
            (Value::Float(a), Value::Float(b)) => match op {
                BinaryOperator::Eq => (a - b).abs() < f64::EPSILON,
                BinaryOperator::Ne => (a - b).abs() >= f64::EPSILON,
                BinaryOperator::Lt => a < b,
                BinaryOperator::Le => a <= b,
                BinaryOperator::Gt => a > b,
                BinaryOperator::Ge => a >= b,
            },
            (Value::String(a), Value::String(b)) => match op {
                BinaryOperator::Eq => a == b,
                BinaryOperator::Ne => a != b,
                BinaryOperator::Lt => a < b,
                BinaryOperator::Le => a <= b,
                BinaryOperator::Gt => a > b,
                BinaryOperator::Ge => a >= b,
            },
            (Value::Boolean(a), Value::Boolean(b)) => match op {
                BinaryOperator::Eq => a == b,
                BinaryOperator::Ne => a != b,
                _ => false,
            },
            (Value::Null, Value::Null) => matches!(op, BinaryOperator::Eq),
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Null => write!(f, "NULL"),
        }
    }
}

/// Query execution context
#[derive(Clone, Default)]
pub struct ExecutionContext {
    /// Storage backend access (simplified - would integrate with actual storage)
    pub data: HashMap<String, Vec<Row>>,
    /// Index access (simplified)
    pub indexes: HashMap<String, HashMap<Vec<u8>, Vec<u64>>>,
}

impl ExecutionContext {
    /// Creates a new execution context
    pub fn new() -> Self {
        Self::default()
    }
}

/// Query executor
pub struct Executor {
    context: ExecutionContext,
}

impl Executor {
    /// Create new executor
    pub fn new(context: ExecutionContext) -> Self {
        Self { context }
    }

    /// Execute a physical plan
    pub fn execute(&mut self, plan: &PhysicalPlan) -> Result<Vec<Row>> {
        self.execute_operator(&plan.root)
    }

    fn execute_operator(&mut self, op: &PhysicalOperator) -> Result<Vec<Row>> {
        match op {
            PhysicalOperator::TableScan { table } => self.execute_table_scan(table),
            PhysicalOperator::IndexScan { table, index, key } => {
                self.execute_index_scan(table, index, key)
            }
            PhysicalOperator::IndexRangeScan {
                table,
                index,
                start,
                end,
            } => self.execute_index_range_scan(table, index, start.as_deref(), end.as_deref()),
            PhysicalOperator::Filter { input, condition } => self.execute_filter(input, condition),
            PhysicalOperator::Sort { input, columns } => self.execute_sort(input, columns),
            PhysicalOperator::Limit {
                input,
                count,
                offset,
            } => self.execute_limit(input, *count, *offset),
            PhysicalOperator::Project { input, columns } => self.execute_project(input, columns),
            PhysicalOperator::HashJoin {
                left,
                right,
                join_type,
                condition,
            } => self.execute_hash_join(left, right, join_type, condition),
            PhysicalOperator::Aggregate { input, aggregates } => {
                self.execute_aggregate(input, aggregates)
            }
        }
    }

    fn execute_table_scan(&mut self, table: &str) -> Result<Vec<Row>> {
        // Get all rows from table
        Ok(self.context.data.get(table).cloned().unwrap_or_default())
    }

    fn execute_index_scan(&mut self, table: &str, index: &str, key: &[u8]) -> Result<Vec<Row>> {
        // Look up row IDs from index
        let row_ids = self
            .context
            .indexes
            .get(index)
            .and_then(|idx| idx.get(key))
            .cloned()
            .unwrap_or_default();

        // Fetch rows by ID
        let all_rows = self.context.data.get(table).cloned().unwrap_or_default();
        let result = row_ids
            .iter()
            .filter_map(|&id| all_rows.get(id as usize).cloned())
            .collect();

        Ok(result)
    }

    fn execute_index_range_scan(
        &mut self,
        table: &str,
        index: &str,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
    ) -> Result<Vec<Row>> {
        // Get all keys from index in range
        let index_data = self.context.indexes.get(index).cloned().unwrap_or_default();

        let mut row_ids = Vec::new();
        for (key, ids) in index_data {
            let in_range = match (start, end) {
                (Some(s), Some(e)) => key.as_slice() >= s && key.as_slice() <= e,
                (Some(s), None) => key.as_slice() >= s,
                (None, Some(e)) => key.as_slice() <= e,
                (None, None) => true,
            };

            if in_range {
                row_ids.extend(ids);
            }
        }

        // Fetch rows by ID
        let all_rows = self.context.data.get(table).cloned().unwrap_or_default();
        let result = row_ids
            .iter()
            .filter_map(|&id| all_rows.get(id as usize).cloned())
            .collect();

        Ok(result)
    }

    fn execute_filter(
        &mut self,
        input: &PhysicalOperator,
        condition: &Expression,
    ) -> Result<Vec<Row>> {
        let rows = self.execute_operator(input)?;

        let filtered = rows
            .into_iter()
            .filter(|row| self.evaluate_condition(row, condition))
            .collect();

        Ok(filtered)
    }

    fn execute_sort(
        &mut self,
        input: &PhysicalOperator,
        columns: &[OrderByColumn],
    ) -> Result<Vec<Row>> {
        let mut rows = self.execute_operator(input)?;

        rows.sort_by(|a, b| {
            for col in columns {
                let a_idx = a.columns.iter().position(|c| c.name == col.column);
                let b_idx = b.columns.iter().position(|c| c.name == col.column);

                if let (Some(a_idx), Some(b_idx)) = (a_idx, b_idx) {
                    let ordering = match (&a.values[a_idx], &b.values[b_idx]) {
                        (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
                        (Value::Float(a), Value::Float(b)) => {
                            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        (Value::String(a), Value::String(b)) => a.cmp(b),
                        (Value::Boolean(a), Value::Boolean(b)) => a.cmp(b),
                        _ => std::cmp::Ordering::Equal,
                    };

                    let ordering = match col.direction {
                        OrderDirection::Asc => ordering,
                        OrderDirection::Desc => ordering.reverse(),
                    };

                    if ordering != std::cmp::Ordering::Equal {
                        return ordering;
                    }
                }
            }
            std::cmp::Ordering::Equal
        });

        Ok(rows)
    }

    fn execute_limit(
        &mut self,
        input: &PhysicalOperator,
        count: usize,
        offset: usize,
    ) -> Result<Vec<Row>> {
        let rows = self.execute_operator(input)?;
        Ok(rows.into_iter().skip(offset).take(count).collect())
    }

    fn execute_project(
        &mut self,
        input: &PhysicalOperator,
        columns: &[SelectColumn],
    ) -> Result<Vec<Row>> {
        let rows = self.execute_operator(input)?;

        let projected = rows
            .into_iter()
            .map(|row| {
                let mut new_columns = Vec::new();
                let mut new_values = Vec::new();

                for col in columns {
                    match col {
                        SelectColumn::Wildcard => {
                            new_columns.extend(row.columns.clone());
                            new_values.extend(row.values.clone());
                        }
                        SelectColumn::Column { name, alias } => {
                            if let Some(idx) = row.columns.iter().position(|c| &c.name == name) {
                                new_columns.push(Column {
                                    name: name.clone(),
                                    alias: alias.clone(),
                                });
                                new_values.push(row.values[idx].clone());
                            }
                        }
                        SelectColumn::Aggregate { .. } => {
                            // Aggregates handled by Aggregate operator
                        }
                    }
                }

                Row {
                    columns: new_columns,
                    values: new_values,
                }
            })
            .collect();

        Ok(projected)
    }

    fn execute_hash_join(
        &mut self,
        left: &PhysicalOperator,
        right: &PhysicalOperator,
        _join_type: &JoinType,
        _condition: &Expression,
    ) -> Result<Vec<Row>> {
        let left_rows = self.execute_operator(left)?;
        let right_rows = self.execute_operator(right)?;

        // Simplified join - cartesian product
        // In a real implementation, this would use the condition to match rows
        let mut result = Vec::new();
        for l_row in &left_rows {
            for r_row in &right_rows {
                let mut columns = l_row.columns.clone();
                columns.extend(r_row.columns.clone());
                let mut values = l_row.values.clone();
                values.extend(r_row.values.clone());

                result.push(Row { columns, values });
            }
        }

        Ok(result)
    }

    fn execute_aggregate(
        &mut self,
        input: &PhysicalOperator,
        aggregates: &[SelectColumn],
    ) -> Result<Vec<Row>> {
        let rows = self.execute_operator(input)?;

        let mut result_columns = Vec::new();
        let mut result_values = Vec::new();

        for agg in aggregates {
            if let SelectColumn::Aggregate {
                function,
                column,
                alias,
            } = agg
            {
                let col_name = match column.as_ref() {
                    SelectColumn::Wildcard => "*",
                    SelectColumn::Column { name, .. } => name.as_str(),
                    _ => continue,
                };

                let value = match function {
                    AggregateFunction::Count => Value::Integer(rows.len() as i64),
                    AggregateFunction::Sum => {
                        let col_idx = rows[0].columns.iter().position(|c| c.name == col_name);
                        if let Some(idx) = col_idx {
                            let sum: i64 = rows
                                .iter()
                                .filter_map(|r| match &r.values[idx] {
                                    Value::Integer(i) => Some(i),
                                    _ => None,
                                })
                                .sum();
                            Value::Integer(sum)
                        } else {
                            Value::Null
                        }
                    }
                    AggregateFunction::Avg => {
                        let col_idx = rows[0].columns.iter().position(|c| c.name == col_name);
                        if let Some(idx) = col_idx {
                            let values: Vec<i64> = rows
                                .iter()
                                .filter_map(|r| match &r.values[idx] {
                                    Value::Integer(i) => Some(*i),
                                    _ => None,
                                })
                                .collect();
                            if !values.is_empty() {
                                let sum: i64 = values.iter().sum();
                                Value::Float(sum as f64 / values.len() as f64)
                            } else {
                                Value::Null
                            }
                        } else {
                            Value::Null
                        }
                    }
                    AggregateFunction::Min => {
                        let col_idx = rows[0].columns.iter().position(|c| c.name == col_name);
                        if let Some(idx) = col_idx {
                            rows.iter()
                                .map(|r| &r.values[idx])
                                .min_by(|a, b| match (a, b) {
                                    (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
                                    _ => std::cmp::Ordering::Equal,
                                })
                                .cloned()
                                .unwrap_or(Value::Null)
                        } else {
                            Value::Null
                        }
                    }
                    AggregateFunction::Max => {
                        let col_idx = rows[0].columns.iter().position(|c| c.name == col_name);
                        if let Some(idx) = col_idx {
                            rows.iter()
                                .map(|r| &r.values[idx])
                                .max_by(|a, b| match (a, b) {
                                    (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
                                    _ => std::cmp::Ordering::Equal,
                                })
                                .cloned()
                                .unwrap_or(Value::Null)
                        } else {
                            Value::Null
                        }
                    }
                };

                let display_name = alias
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| format!("{}({})", function, col_name));

                result_columns.push(Column {
                    name: display_name.clone(),
                    alias: alias.clone(),
                });
                result_values.push(value);
            }
        }

        Ok(vec![Row {
            columns: result_columns,
            values: result_values,
        }])
    }

    fn evaluate_condition(&self, row: &Row, condition: &Expression) -> bool {
        match condition {
            Expression::Column(name) => {
                // Column reference - check if exists and is truthy
                row.columns.iter().any(|c| &c.name == name)
            }
            Expression::Literal(lit) => {
                // Literal value
                match lit {
                    Literal::Boolean(b) => *b,
                    _ => true,
                }
            }
            Expression::BinaryOp { left, op, right } => {
                let left_val = self.evaluate_expression(row, left);
                let right_val = self.evaluate_expression(row, right);

                if let (Some(l), Some(r)) = (left_val, right_val) {
                    l.compare(&r, op)
                } else {
                    false
                }
            }
            Expression::LogicalOp { left, op, right } => {
                let left_result = self.evaluate_condition(row, left);
                let right_result = self.evaluate_condition(row, right);

                match op {
                    LogicalOperator::And => left_result && right_result,
                    LogicalOperator::Or => left_result || right_result,
                }
            }
            Expression::Not(expr) => !self.evaluate_condition(row, expr),
            Expression::Like { expr, pattern } => {
                if let Some(Value::String(s)) = self.evaluate_expression(row, expr) {
                    // Simplified LIKE - just use contains for now
                    let pattern = pattern.replace('%', "");
                    s.contains(&pattern)
                } else {
                    false
                }
            }
            Expression::In { expr, values } => {
                self.evaluate_expression(row, expr).is_some_and(|val| {
                    values.iter().any(|lit| {
                        let lit_val = literal_to_value(lit);
                        val == lit_val
                    })
                })
            }
            Expression::Between { expr, min, max } => {
                if let (Some(val), Some(min_v), Some(max_v)) = (
                    self.evaluate_expression(row, expr),
                    self.evaluate_expression(row, min),
                    self.evaluate_expression(row, max),
                ) {
                    val.compare(&min_v, &BinaryOperator::Ge)
                        && val.compare(&max_v, &BinaryOperator::Le)
                } else {
                    false
                }
            }
        }
    }

    fn evaluate_expression(&self, row: &Row, expr: &Expression) -> Option<Value> {
        match expr {
            Expression::Column(name) => row
                .columns
                .iter()
                .position(|c| &c.name == name)
                .and_then(|idx| row.values.get(idx).cloned()),
            Expression::Literal(lit) => Some(literal_to_value(lit)),
            _ => None,
        }
    }
}

fn literal_to_value(lit: &Literal) -> Value {
    match lit {
        Literal::Integer(i) => Value::Integer(*i),
        Literal::Float(f) => Value::Float(*f),
        Literal::String(s) => Value::String(s.clone()),
        Literal::Boolean(b) => Value::Boolean(*b),
        Literal::Null => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::parser::Parser;
    use crate::query::planner::Planner;

    #[test]
    fn test_table_scan() {
        let mut context = ExecutionContext::new();
        context.data.insert(
            "users".to_string(),
            vec![
                Row {
                    columns: vec![
                        Column {
                            name: "id".to_string(),
                            alias: None,
                        },
                        Column {
                            name: "name".to_string(),
                            alias: None,
                        },
                    ],
                    values: vec![Value::Integer(1), Value::String("Alice".to_string())],
                },
                Row {
                    columns: vec![
                        Column {
                            name: "id".to_string(),
                            alias: None,
                        },
                        Column {
                            name: "name".to_string(),
                            alias: None,
                        },
                    ],
                    values: vec![Value::Integer(2), Value::String("Bob".to_string())],
                },
            ],
        );

        let mut executor = Executor::new(context);

        let mut parser = Parser::new("SELECT * FROM users").unwrap();
        let query = parser.parse().unwrap();
        let planner = Planner::new();
        let plan = planner.plan(&query).unwrap();

        let result = executor.execute(&plan).unwrap();
        assert_eq!(result.len(), 2);
    }
}
