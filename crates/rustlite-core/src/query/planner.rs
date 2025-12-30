/// Query planner and optimizer
///
/// Converts AST into optimized physical execution plans.
use super::ast::*;
use std::fmt;

/// Physical query plan
#[derive(Debug, Clone)]
pub struct PhysicalPlan {
    pub root: PhysicalOperator,
}

/// Physical operators for query execution
#[derive(Debug, Clone)]
pub enum PhysicalOperator {
    /// Full table scan
    TableScan { table: String },
    /// Index scan with exact match
    IndexScan {
        table: String,
        index: String,
        key: Vec<u8>,
    },
    /// Index range scan
    IndexRangeScan {
        table: String,
        index: String,
        start: Option<Vec<u8>>,
        end: Option<Vec<u8>>,
    },
    /// Filter rows based on predicate
    Filter {
        input: Box<PhysicalOperator>,
        condition: Expression,
    },
    /// Sort rows
    Sort {
        input: Box<PhysicalOperator>,
        columns: Vec<OrderByColumn>,
    },
    /// Limit number of results
    Limit {
        input: Box<PhysicalOperator>,
        count: usize,
        offset: usize,
    },
    /// Project columns (SELECT specific columns)
    Project {
        input: Box<PhysicalOperator>,
        columns: Vec<SelectColumn>,
    },
    /// Hash join (inner, left, right, full)
    HashJoin {
        left: Box<PhysicalOperator>,
        right: Box<PhysicalOperator>,
        join_type: JoinType,
        condition: Expression,
    },
    /// GROUP BY with optional aggregation
    GroupBy {
        input: Box<PhysicalOperator>,
        group_columns: Vec<String>,
        aggregates: Vec<SelectColumn>,
        having: Option<Expression>,
    },
    /// Aggregation (COUNT, SUM, AVG, MIN, MAX) without grouping
    Aggregate {
        input: Box<PhysicalOperator>,
        aggregates: Vec<SelectColumn>,
    },
}

/// Query planner
pub struct Planner {
    /// Available indexes for optimization
    available_indexes: Vec<IndexMetadata>,
}

/// Metadata about available indexes
#[derive(Debug, Clone)]
pub struct IndexMetadata {
    pub name: String,
    pub table: String,
    pub index_type: String, // "BTree" or "Hash"
}

impl Planner {
    /// Create a new planner
    pub fn new() -> Self {
        Self {
            available_indexes: Vec::new(),
        }
    }

    /// Create planner with known indexes
    pub fn with_indexes(indexes: Vec<IndexMetadata>) -> Self {
        Self {
            available_indexes: indexes,
        }
    }

    /// Plan a query
    pub fn plan(&self, query: &Query) -> Result<PhysicalPlan, PlanError> {
        // Start with base table access
        let mut plan = self.plan_table_access(&query.from)?;

        // Apply WHERE clause (predicate pushdown)
        if let Some(ref where_clause) = query.where_clause {
            plan = self.apply_filter(plan, &where_clause.condition)?;
        }

        // Check if we have aggregates or GROUP BY
        let has_aggregates = query
            .select
            .columns
            .iter()
            .any(|col| matches!(col, SelectColumn::Aggregate { .. }));

        // Apply GROUP BY and aggregation if needed
        if let Some(ref group_by) = query.group_by {
            // For GROUP BY, we need:
            // 1. All columns used in GROUP BY
            // 2. All columns referenced in aggregate functions
            // We'll just pass through all columns (TableScan) and let GroupBy handle it

            // Apply GROUP BY with aggregates
            plan = PhysicalOperator::GroupBy {
                input: Box::new(plan),
                group_columns: group_by.columns.clone(),
                aggregates: query.select.columns.clone(),
                having: query.having.as_ref().map(|h| h.condition.clone()),
            };
        } else if has_aggregates {
            // For aggregation without GROUP BY, we also need all referenced columns
            // Pass through TableScan directly

            // Aggregation without GROUP BY
            plan = PhysicalOperator::Aggregate {
                input: Box::new(plan),
                aggregates: query.select.columns.clone(),
            };
        } else {
            // No aggregates - normal projection
            plan = PhysicalOperator::Project {
                input: Box::new(plan),
                columns: query.select.columns.clone(),
            };
        }

        // Apply ORDER BY
        if let Some(ref order_by) = query.order_by {
            plan = PhysicalOperator::Sort {
                input: Box::new(plan),
                columns: order_by.columns.clone(),
            };
        }

        // Apply LIMIT
        if let Some(ref limit) = query.limit {
            plan = PhysicalOperator::Limit {
                input: Box::new(plan),
                count: limit.count,
                offset: limit.offset.unwrap_or(0),
            };
        }

        Ok(PhysicalPlan { root: plan })
    }

    fn plan_table_access(&self, from: &FromClause) -> Result<PhysicalOperator, PlanError> {
        let mut plan = PhysicalOperator::TableScan {
            table: from.table.clone(),
        };

        // Plan JOINs
        for join in &from.joins {
            let right = PhysicalOperator::TableScan {
                table: join.table.clone(),
            };

            plan = PhysicalOperator::HashJoin {
                left: Box::new(plan),
                right: Box::new(right),
                join_type: join.join_type.clone(),
                condition: join.condition.clone(),
            };
        }

        Ok(plan)
    }

    fn apply_filter(
        &self,
        input: PhysicalOperator,
        condition: &Expression,
    ) -> Result<PhysicalOperator, PlanError> {
        // Try to use index if available
        if let Some(index_scan) = self.try_index_scan(condition) {
            return Ok(index_scan);
        }

        // Otherwise, use filter operator
        Ok(PhysicalOperator::Filter {
            input: Box::new(input),
            condition: condition.clone(),
        })
    }

    fn try_index_scan(&self, condition: &Expression) -> Option<PhysicalOperator> {
        // Check if condition can use an index
        match condition {
            Expression::BinaryOp { left, op, right } => {
                // Extract column name and value
                let (column, value) = match (left.as_ref(), right.as_ref()) {
                    (Expression::Column(col), Expression::Literal(lit)) => (col, lit),
                    _ => return None,
                };

                // Find matching index
                for index in &self.available_indexes {
                    // Simplified: assume index name contains column name
                    if index.name.contains(column) {
                        match index.index_type.as_str() {
                            "Hash" if *op == BinaryOperator::Eq => {
                                // Use hash index for exact match
                                return Some(PhysicalOperator::IndexScan {
                                    table: index.table.clone(),
                                    index: index.name.clone(),
                                    key: literal_to_bytes(value),
                                });
                            }
                            "BTree" => {
                                // Use B-Tree index for range queries
                                match op {
                                    BinaryOperator::Eq => {
                                        return Some(PhysicalOperator::IndexScan {
                                            table: index.table.clone(),
                                            index: index.name.clone(),
                                            key: literal_to_bytes(value),
                                        });
                                    }
                                    BinaryOperator::Lt | BinaryOperator::Le => {
                                        return Some(PhysicalOperator::IndexRangeScan {
                                            table: index.table.clone(),
                                            index: index.name.clone(),
                                            start: None,
                                            end: Some(literal_to_bytes(value)),
                                        });
                                    }
                                    BinaryOperator::Gt | BinaryOperator::Ge => {
                                        return Some(PhysicalOperator::IndexRangeScan {
                                            table: index.table.clone(),
                                            index: index.name.clone(),
                                            start: Some(literal_to_bytes(value)),
                                            end: None,
                                        });
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Expression::Between { expr, min, max } => {
                // Extract column name
                let column = match expr.as_ref() {
                    Expression::Column(col) => col,
                    _ => return None,
                };

                // Find matching B-Tree index
                for index in &self.available_indexes {
                    if index.index_type == "BTree" && index.name.contains(column) {
                        let start = match min.as_ref() {
                            Expression::Literal(lit) => Some(literal_to_bytes(lit)),
                            _ => None,
                        };
                        let end = match max.as_ref() {
                            Expression::Literal(lit) => Some(literal_to_bytes(lit)),
                            _ => None,
                        };

                        return Some(PhysicalOperator::IndexRangeScan {
                            table: index.table.clone(),
                            index: index.name.clone(),
                            start,
                            end,
                        });
                    }
                }
            }
            _ => {}
        }

        None
    }
}

impl Default for Planner {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert literal to bytes for index lookup
fn literal_to_bytes(literal: &Literal) -> Vec<u8> {
    match literal {
        Literal::Integer(i) => i.to_le_bytes().to_vec(),
        Literal::Float(f) => f.to_le_bytes().to_vec(),
        Literal::String(s) => s.as_bytes().to_vec(),
        Literal::Boolean(b) => vec![if *b { 1 } else { 0 }],
        Literal::Null => vec![],
    }
}

/// Planning errors
#[derive(Debug, Clone)]
pub enum PlanError {
    UnsupportedOperation(String),
    InvalidExpression(String),
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanError::UnsupportedOperation(op) => write!(f, "Unsupported operation: {}", op),
            PlanError::InvalidExpression(expr) => write!(f, "Invalid expression: {}", expr),
        }
    }
}

impl std::error::Error for PlanError {}

impl fmt::Display for PhysicalPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.root)
    }
}

impl fmt::Display for PhysicalOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhysicalOperator::TableScan { table } => write!(f, "TableScan({})", table),
            PhysicalOperator::IndexScan { table, index, .. } => {
                write!(f, "IndexScan({}.{})", table, index)
            }
            PhysicalOperator::IndexRangeScan { table, index, .. } => {
                write!(f, "IndexRangeScan({}.{})", table, index)
            }
            PhysicalOperator::Filter { input, condition } => {
                write!(f, "Filter({}) -> {}", condition, input)
            }
            PhysicalOperator::Sort { input, columns } => {
                write!(f, "Sort(")?;
                for (i, col) in columns.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", col)?;
                }
                write!(f, ") -> {}", input)
            }
            PhysicalOperator::Limit {
                input,
                count,
                offset,
            } => {
                write!(f, "Limit({}, {}) -> {}", count, offset, input)
            }
            PhysicalOperator::Project { input, columns } => {
                write!(f, "Project(")?;
                for (i, col) in columns.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", col)?;
                }
                write!(f, ") -> {}", input)
            }
            PhysicalOperator::HashJoin {
                left,
                right,
                join_type,
                ..
            } => {
                write!(f, "{}Join({} x {})", join_type, left, right)
            }
            PhysicalOperator::GroupBy {
                input,
                group_columns,
                aggregates,
                having,
            } => {
                write!(f, "GroupBy(")?;
                for (i, col) in group_columns.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", col)?;
                }
                if !aggregates.is_empty() {
                    write!(f, " | ")?;
                    for (i, agg) in aggregates.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", agg)?;
                    }
                }
                if let Some(h) = having {
                    write!(f, " HAVING {}", h)?;
                }
                write!(f, ") -> {}", input)
            }
            PhysicalOperator::Aggregate { input, aggregates } => {
                write!(f, "Aggregate(")?;
                for (i, agg) in aggregates.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", agg)?;
                }
                write!(f, ") -> {}", input)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::parser::Parser;

    #[test]
    fn test_simple_plan() {
        let mut parser = Parser::new("SELECT * FROM users").unwrap();
        let query = parser.parse().unwrap();

        let planner = Planner::new();
        let plan = planner.plan(&query).unwrap();

        // Should have Project -> TableScan
        match plan.root {
            PhysicalOperator::Project { input, .. } => match *input {
                PhysicalOperator::TableScan { .. } => {}
                _ => panic!("Expected TableScan"),
            },
            _ => panic!("Expected Project"),
        }
    }

    #[test]
    fn test_filter_plan() {
        let mut parser = Parser::new("SELECT * FROM users WHERE age > 18").unwrap();
        let query = parser.parse().unwrap();

        let planner = Planner::new();
        let plan = planner.plan(&query).unwrap();

        // Should have Project -> Filter -> TableScan
        match plan.root {
            PhysicalOperator::Project { input, .. } => match *input {
                PhysicalOperator::Filter { .. } => {}
                _ => panic!("Expected Filter"),
            },
            _ => panic!("Expected Project"),
        }
    }

    #[test]
    fn test_order_by_plan() {
        let mut parser = Parser::new("SELECT * FROM users ORDER BY name").unwrap();
        let query = parser.parse().unwrap();

        let planner = Planner::new();
        let plan = planner.plan(&query).unwrap();

        // Should have Sort somewhere in the plan
        let plan_str = format!("{}", plan);
        assert!(plan_str.contains("Sort"));
    }

    #[test]
    fn test_limit_plan() {
        let mut parser = Parser::new("SELECT * FROM users LIMIT 10").unwrap();
        let query = parser.parse().unwrap();

        let planner = Planner::new();
        let plan = planner.plan(&query).unwrap();

        // Should have Limit somewhere in the plan
        let plan_str = format!("{}", plan);
        assert!(plan_str.contains("Limit"));
    }
}
