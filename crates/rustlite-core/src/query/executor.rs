/// Query executor
///
/// Executes physical query plans using iterators.
use super::ast::*;
use super::planner::{PhysicalOperator, PhysicalPlan};
use crate::error::Result;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Hashable wrapper for group key values
#[derive(Debug, Clone, Eq)]
struct GroupKey(Vec<GroupValue>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum GroupValue {
    Integer(i64),
    Float(i64), // Store float as bits for hashing
    String(String),
    Boolean(bool),
    Null,
}

impl From<&Value> for GroupValue {
    fn from(value: &Value) -> Self {
        match value {
            Value::Integer(i) => GroupValue::Integer(*i),
            Value::Float(f) => GroupValue::Float(f.to_bits() as i64),
            Value::String(s) => GroupValue::String(s.clone()),
            Value::Boolean(b) => GroupValue::Boolean(*b),
            Value::Null => GroupValue::Null,
        }
    }
}

impl PartialEq for GroupKey {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Hash for GroupKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

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
            PhysicalOperator::GroupBy {
                input,
                group_columns,
                aggregates,
                having,
            } => self.execute_group_by(input, group_columns, aggregates, having.as_ref()),
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
        join_type: &JoinType,
        condition: &Expression,
    ) -> Result<Vec<Row>> {
        let left_rows = self.execute_operator(left)?;
        let right_rows = self.execute_operator(right)?;

        // Choose join algorithm based on dataset size
        if right_rows.len() < 100 {
            // Use nested loop join for small datasets
            self.nested_loop_join(&left_rows, &right_rows, join_type, condition)
        } else {
            // Use hash join for larger datasets
            self.hash_join_impl(&left_rows, &right_rows, join_type, condition)
        }
    }

    /// Nested loop join - simple but works for small datasets
    fn nested_loop_join(
        &mut self,
        left_rows: &[Row],
        right_rows: &[Row],
        join_type: &JoinType,
        condition: &Expression,
    ) -> Result<Vec<Row>> {
        let mut result = Vec::new();

        match join_type {
            JoinType::Inner => {
                for l_row in left_rows {
                    for r_row in right_rows {
                        if self.evaluate_join_condition(l_row, r_row, condition) {
                            result.push(self.merge_rows(l_row, r_row));
                        }
                    }
                }
            }
            JoinType::Left => {
                for l_row in left_rows {
                    let mut matched = false;
                    for r_row in right_rows {
                        if self.evaluate_join_condition(l_row, r_row, condition) {
                            result.push(self.merge_rows(l_row, r_row));
                            matched = true;
                        }
                    }
                    if !matched {
                        // Left row with NULL values for right side
                        result.push(self.merge_rows_with_null(l_row, right_rows[0].columns.len()));
                    }
                }
            }
            JoinType::Right => {
                for r_row in right_rows {
                    let mut matched = false;
                    for l_row in left_rows {
                        if self.evaluate_join_condition(l_row, r_row, condition) {
                            result.push(self.merge_rows(l_row, r_row));
                            matched = true;
                        }
                    }
                    if !matched {
                        // NULL values for left side with right row
                        result.push(self.merge_null_with_row(left_rows[0].columns.len(), r_row));
                    }
                }
            }
            JoinType::Full => {
                let mut left_matched = vec![false; left_rows.len()];
                let mut right_matched = vec![false; right_rows.len()];

                for (l_idx, l_row) in left_rows.iter().enumerate() {
                    for (r_idx, r_row) in right_rows.iter().enumerate() {
                        if self.evaluate_join_condition(l_row, r_row, condition) {
                            result.push(self.merge_rows(l_row, r_row));
                            left_matched[l_idx] = true;
                            right_matched[r_idx] = true;
                        }
                    }
                }

                // Add unmatched left rows
                for (idx, matched) in left_matched.iter().enumerate() {
                    if !*matched {
                        result.push(
                            self.merge_rows_with_null(&left_rows[idx], right_rows[0].columns.len()),
                        );
                    }
                }

                // Add unmatched right rows
                for (idx, matched) in right_matched.iter().enumerate() {
                    if !*matched {
                        result.push(
                            self.merge_null_with_row(left_rows[0].columns.len(), &right_rows[idx]),
                        );
                    }
                }
            }
        }

        Ok(result)
    }

    /// Hash join - efficient for larger datasets
    fn hash_join_impl(
        &mut self,
        left_rows: &[Row],
        right_rows: &[Row],
        join_type: &JoinType,
        condition: &Expression,
    ) -> Result<Vec<Row>> {
        // Build hash table from right side (build phase)
        let mut hash_table: HashMap<Vec<u8>, Vec<&Row>> = HashMap::new();

        for r_row in right_rows {
            let key = self.extract_join_key(r_row, condition, true);
            hash_table.entry(key).or_default().push(r_row);
        }

        let mut result = Vec::new();

        match join_type {
            JoinType::Inner => {
                for l_row in left_rows {
                    let key = self.extract_join_key(l_row, condition, false);
                    if let Some(matching_rows) = hash_table.get(&key) {
                        for r_row in matching_rows {
                            if self.evaluate_join_condition(l_row, r_row, condition) {
                                result.push(self.merge_rows(l_row, r_row));
                            }
                        }
                    }
                }
            }
            JoinType::Left => {
                for l_row in left_rows {
                    let key = self.extract_join_key(l_row, condition, false);
                    if let Some(matching_rows) = hash_table.get(&key) {
                        let mut matched = false;
                        for r_row in matching_rows {
                            if self.evaluate_join_condition(l_row, r_row, condition) {
                                result.push(self.merge_rows(l_row, r_row));
                                matched = true;
                            }
                        }
                        if !matched {
                            result.push(
                                self.merge_rows_with_null(l_row, right_rows[0].columns.len()),
                            );
                        }
                    } else {
                        result.push(self.merge_rows_with_null(l_row, right_rows[0].columns.len()));
                    }
                }
            }
            JoinType::Right | JoinType::Full => {
                // For RIGHT and FULL, fall back to nested loop
                // (hash join is less efficient for these join types)
                return self.nested_loop_join(left_rows, right_rows, join_type, condition);
            }
        }

        Ok(result)
    }

    /// Extract join key from row for hashing
    fn extract_join_key(&self, row: &Row, condition: &Expression, is_right: bool) -> Vec<u8> {
        // Simple key extraction - would be more sophisticated in production
        if let Expression::BinaryOp { left, right, .. } = condition {
            if let (Expression::Column(left_col), Expression::Column(right_col)) =
                (left.as_ref(), right.as_ref())
            {
                let col_name = if is_right { right_col } else { left_col };

                // Strip table prefix if present (e.g., "users.id" -> "id")
                let column_name = if let Some(dot_pos) = col_name.rfind('.') {
                    &col_name[dot_pos + 1..]
                } else {
                    col_name
                };

                if let Some(idx) = row.columns.iter().position(|c| c.name == column_name) {
                    return row.values[idx].to_bytes();
                }
            }
        }
        vec![]
    }

    /// Evaluate join condition for two rows
    fn evaluate_join_condition(&self, left: &Row, right: &Row, condition: &Expression) -> bool {
        match condition {
            Expression::BinaryOp {
                left: l_expr,
                op,
                right: r_expr,
            } => {
                let left_val = self.evaluate_expression_for_row(l_expr, left, right, true);
                let right_val = self.evaluate_expression_for_row(r_expr, left, right, false);

                if let (Some(lv), Some(rv)) = (left_val, right_val) {
                    return lv.compare(&rv, op);
                }
                false
            }
            Expression::LogicalOp {
                left: l_expr,
                op,
                right: r_expr,
            } => {
                let left_result = self.evaluate_join_condition(left, right, l_expr);
                let right_result = self.evaluate_join_condition(left, right, r_expr);

                match op {
                    LogicalOperator::And => left_result && right_result,
                    LogicalOperator::Or => left_result || right_result,
                }
            }
            _ => true, // Default to true for unsupported conditions
        }
    }

    /// Evaluate expression in the context of two joined rows
    fn evaluate_expression_for_row(
        &self,
        expr: &Expression,
        left_row: &Row,
        right_row: &Row,
        is_left: bool,
    ) -> Option<Value> {
        match expr {
            Expression::Column(name) => {
                // Strip table prefix if present (e.g., "users.id" -> "id")
                let column_name = if let Some(dot_pos) = name.rfind('.') {
                    &name[dot_pos + 1..]
                } else {
                    name
                };

                // Try to find column in appropriate row
                let row = if is_left { left_row } else { right_row };
                row.columns
                    .iter()
                    .position(|c| c.name == column_name)
                    .map(|idx| row.values[idx].clone())
            }
            Expression::Literal(lit) => Some(self.literal_to_value(lit)),
            _ => None,
        }
    }

    /// Merge two rows into one
    fn merge_rows(&self, left: &Row, right: &Row) -> Row {
        let mut columns = left.columns.clone();
        columns.extend(right.columns.clone());
        let mut values = left.values.clone();
        values.extend(right.values.clone());
        Row { columns, values }
    }

    /// Merge left row with NULL values for right side
    fn merge_rows_with_null(&self, left: &Row, right_col_count: usize) -> Row {
        let columns = left.columns.clone();
        let mut values = left.values.clone();
        for _ in 0..right_col_count {
            values.push(Value::Null);
        }
        Row { columns, values }
    }

    /// Merge NULL values for left side with right row
    fn merge_null_with_row(&self, left_col_count: usize, right: &Row) -> Row {
        let mut columns = Vec::new();
        let mut values = Vec::new();
        for _ in 0..left_col_count {
            values.push(Value::Null);
        }
        columns.extend(right.columns.clone());
        values.extend(right.values.clone());
        Row { columns, values }
    }

    /// Convert literal to value
    fn literal_to_value(&self, lit: &Literal) -> Value {
        match lit {
            Literal::Integer(i) => Value::Integer(*i),
            Literal::Float(f) => Value::Float(*f),
            Literal::String(s) => Value::String(s.clone()),
            Literal::Boolean(b) => Value::Boolean(*b),
            Literal::Null => Value::Null,
        }
    }

    fn execute_group_by(
        &mut self,
        input: &PhysicalOperator,
        group_columns: &[String],
        aggregates: &[SelectColumn],
        having: Option<&Expression>,
    ) -> Result<Vec<Row>> {
        let rows = self.execute_operator(input)?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        // Group rows by the specified columns
        let mut groups: HashMap<GroupKey, Vec<Row>> = HashMap::new();

        for row in rows {
            // Extract group key values
            let mut key_values = Vec::new();
            for group_col in group_columns {
                if let Some(col_idx) = row.columns.iter().position(|c| &c.name == group_col) {
                    key_values.push(GroupValue::from(&row.values[col_idx]));
                } else {
                    key_values.push(GroupValue::Null);
                }
            }

            let group_key = GroupKey(key_values);
            groups.entry(group_key).or_default().push(row);
        }

        // Apply aggregates for each group
        let mut result_rows = Vec::new();

        for (group_key, group_rows) in groups {
            let mut result_columns = Vec::new();
            let mut result_values = Vec::new();

            // Add group columns
            for (i, col_name) in group_columns.iter().enumerate() {
                result_columns.push(Column {
                    name: col_name.clone(),
                    alias: None,
                });
                // Convert GroupValue back to Value
                let value = match &group_key.0[i] {
                    GroupValue::Integer(i) => Value::Integer(*i),
                    GroupValue::Float(bits) => Value::Float(f64::from_bits(*bits as u64)),
                    GroupValue::String(s) => Value::String(s.clone()),
                    GroupValue::Boolean(b) => Value::Boolean(*b),
                    GroupValue::Null => Value::Null,
                };
                result_values.push(value);
            }

            // Add aggregate columns
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

                    let value = self.compute_aggregate(function, col_name, &group_rows)?;

                    let display_name = alias
                        .as_ref()
                        .cloned()
                        .unwrap_or_else(|| format!("{}({})", function, col_name));

                    result_columns.push(Column {
                        name: display_name.clone(),
                        alias: alias.clone(),
                    });
                    result_values.push(value);
                } else if let SelectColumn::Column { name, alias: _ } = agg {
                    // Non-aggregate column (must be in GROUP BY)
                    if !group_columns.contains(name) {
                        // This would be a SQL error - column must be in GROUP BY or be aggregated
                        continue;
                    }
                }
            }

            let row = Row {
                columns: result_columns,
                values: result_values,
            };

            // Apply HAVING clause if present
            if let Some(having_condition) = having {
                if self.evaluate_condition(&row, having_condition) {
                    result_rows.push(row);
                }
            } else {
                result_rows.push(row);
            }
        }

        Ok(result_rows)
    }

    fn compute_aggregate(
        &self,
        function: &AggregateFunction,
        col_name: &str,
        rows: &[Row],
    ) -> Result<Value> {
        if rows.is_empty() {
            return Ok(Value::Null);
        }

        match function {
            AggregateFunction::Count => {
                if col_name == "*" {
                    // COUNT(*): Count all rows
                    Ok(Value::Integer(rows.len() as i64))
                } else {
                    // COUNT(column): Count non-null values
                    let col_idx = rows
                        .iter()
                        .find_map(|r| r.columns.iter().position(|c| c.name == col_name));

                    if let Some(idx) = col_idx {
                        let count = rows
                            .iter()
                            .filter(|r| {
                                idx < r.values.len() && !matches!(r.values[idx], Value::Null)
                            })
                            .count();
                        Ok(Value::Integer(count as i64))
                    } else {
                        Ok(Value::Integer(0))
                    }
                }
            }
            AggregateFunction::Sum => {
                // Find column index - check all rows if first doesn't have it
                let col_idx = rows
                    .iter()
                    .find_map(|r| r.columns.iter().position(|c| c.name == col_name));

                if let Some(idx) = col_idx {
                    let sum: i64 = rows
                        .iter()
                        .filter_map(|r| {
                            if idx < r.values.len() {
                                match &r.values[idx] {
                                    Value::Integer(i) => Some(i),
                                    _ => None,
                                }
                            } else {
                                None
                            }
                        })
                        .sum();
                    Ok(Value::Integer(sum))
                } else {
                    Ok(Value::Null)
                }
            }
            AggregateFunction::Avg => {
                let col_idx = rows
                    .iter()
                    .find_map(|r| r.columns.iter().position(|c| c.name == col_name));

                if let Some(idx) = col_idx {
                    let values: Vec<i64> = rows
                        .iter()
                        .filter_map(|r| {
                            if idx < r.values.len() {
                                match &r.values[idx] {
                                    Value::Integer(i) => Some(*i),
                                    _ => None,
                                }
                            } else {
                                None
                            }
                        })
                        .collect();
                    if !values.is_empty() {
                        let sum: i64 = values.iter().sum();
                        Ok(Value::Float(sum as f64 / values.len() as f64))
                    } else {
                        Ok(Value::Null)
                    }
                } else {
                    Ok(Value::Null)
                }
            }
            AggregateFunction::Min => {
                let col_idx = rows
                    .iter()
                    .find_map(|r| r.columns.iter().position(|c| c.name == col_name));

                if let Some(idx) = col_idx {
                    Ok(rows
                        .iter()
                        .filter_map(|r| {
                            if idx < r.values.len() {
                                Some(&r.values[idx])
                            } else {
                                None
                            }
                        })
                        .min_by(|a, b| match (a, b) {
                            (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
                            (Value::Float(a), Value::Float(b)) => {
                                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                            }
                            (Value::String(a), Value::String(b)) => a.cmp(b),
                            _ => std::cmp::Ordering::Equal,
                        })
                        .cloned()
                        .unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            }
            AggregateFunction::Max => {
                let col_idx = rows
                    .iter()
                    .find_map(|r| r.columns.iter().position(|c| c.name == col_name));

                if let Some(idx) = col_idx {
                    Ok(rows
                        .iter()
                        .filter_map(|r| {
                            if idx < r.values.len() {
                                Some(&r.values[idx])
                            } else {
                                None
                            }
                        })
                        .max_by(|a, b| match (a, b) {
                            (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
                            (Value::Float(a), Value::Float(b)) => {
                                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                            }
                            (Value::String(a), Value::String(b)) => a.cmp(b),
                            _ => std::cmp::Ordering::Equal,
                        })
                        .cloned()
                        .unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            }
        }
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

                let value = self.compute_aggregate(function, col_name, &rows)?;

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
