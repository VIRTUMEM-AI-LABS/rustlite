/// Lexer for tokenizing SQL-like queries
///
/// Converts raw SQL text into a stream of tokens for parsing.
use std::fmt;

/// Token types produced by the lexer
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Select,
    From,
    Where,
    Group,
    By,
    Having,
    OrderBy,
    Limit,
    Offset,
    Join,
    Inner,
    Left,
    Right,
    Full,
    On,
    As,
    And,
    Or,
    Not,
    Like,
    In,
    Between,

    // Aggregate functions
    Count,
    Sum,
    Avg,
    Min,
    Max,

    // Operators
    Eq, // =
    Ne, // !=
    Lt, // <
    Le, // <=
    Gt, // >
    Ge, // >=

    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,

    // Identifiers
    Identifier(String),

    // Punctuation
    Asterisk,   // *
    Comma,      // ,
    LeftParen,  // (
    RightParen, // )

    // Special
    Asc,
    Desc,

    // End of input
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Select => write!(f, "SELECT"),
            Token::From => write!(f, "FROM"),
            Token::Where => write!(f, "WHERE"),
            Token::Group => write!(f, "GROUP"),
            Token::By => write!(f, "BY"),
            Token::Having => write!(f, "HAVING"),
            Token::OrderBy => write!(f, "ORDER BY"),
            Token::Limit => write!(f, "LIMIT"),
            Token::Offset => write!(f, "OFFSET"),
            Token::Join => write!(f, "JOIN"),
            Token::Inner => write!(f, "INNER"),
            Token::Left => write!(f, "LEFT"),
            Token::Right => write!(f, "RIGHT"),
            Token::Full => write!(f, "FULL"),
            Token::On => write!(f, "ON"),
            Token::As => write!(f, "AS"),
            Token::And => write!(f, "AND"),
            Token::Or => write!(f, "OR"),
            Token::Not => write!(f, "NOT"),
            Token::Like => write!(f, "LIKE"),
            Token::In => write!(f, "IN"),
            Token::Between => write!(f, "BETWEEN"),
            Token::Count => write!(f, "COUNT"),
            Token::Sum => write!(f, "SUM"),
            Token::Avg => write!(f, "AVG"),
            Token::Min => write!(f, "MIN"),
            Token::Max => write!(f, "MAX"),
            Token::Eq => write!(f, "="),
            Token::Ne => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::Le => write!(f, "<="),
            Token::Gt => write!(f, ">"),
            Token::Ge => write!(f, ">="),
            Token::Integer(i) => write!(f, "{}", i),
            Token::Float(fl) => write!(f, "{}", fl),
            Token::String(s) => write!(f, "'{}'", s),
            Token::Boolean(b) => write!(f, "{}", b),
            Token::Null => write!(f, "NULL"),
            Token::Identifier(id) => write!(f, "{}", id),
            Token::Asterisk => write!(f, "*"),
            Token::Comma => write!(f, ","),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::Asc => write!(f, "ASC"),
            Token::Desc => write!(f, "DESC"),
            Token::Eof => write!(f, "EOF"),
        }
    }
}

/// Lexer state
pub struct Lexer {
    input: Vec<char>,
    position: usize,
}

impl Lexer {
    /// Create a new lexer from input string
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
        }
    }

    /// Get the next token
    pub fn next_token(&mut self) -> Result<Token, LexerError> {
        self.skip_whitespace();

        if self.position >= self.input.len() {
            return Ok(Token::Eof);
        }

        let ch = self.current_char();

        // Single-character tokens
        match ch {
            '*' => {
                self.advance();
                return Ok(Token::Asterisk);
            }
            ',' => {
                self.advance();
                return Ok(Token::Comma);
            }
            '(' => {
                self.advance();
                return Ok(Token::LeftParen);
            }
            ')' => {
                self.advance();
                return Ok(Token::RightParen);
            }
            '=' => {
                self.advance();
                return Ok(Token::Eq);
            }
            '<' => {
                self.advance();
                if self.position < self.input.len() && self.current_char() == '=' {
                    self.advance();
                    return Ok(Token::Le);
                }
                return Ok(Token::Lt);
            }
            '>' => {
                self.advance();
                if self.position < self.input.len() && self.current_char() == '=' {
                    self.advance();
                    return Ok(Token::Ge);
                }
                return Ok(Token::Gt);
            }
            '!' => {
                self.advance();
                if self.position < self.input.len() && self.current_char() == '=' {
                    self.advance();
                    return Ok(Token::Ne);
                }
                return Err(LexerError::UnexpectedCharacter(ch));
            }
            '\'' => return self.read_string(),
            _ => {}
        }

        // Numbers
        if ch.is_ascii_digit() {
            return self.read_number();
        }

        // Identifiers and keywords
        if ch.is_alphabetic() || ch == '_' {
            return self.read_identifier_or_keyword();
        }

        Err(LexerError::UnexpectedCharacter(ch))
    }

    /// Tokenize entire input into vector of tokens
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            if token == Token::Eof {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }
        Ok(tokens)
    }

    fn current_char(&self) -> char {
        self.input[self.position]
    }

    fn peek_char(&self) -> Option<char> {
        if self.position + 1 < self.input.len() {
            Some(self.input[self.position + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.input.len() && self.current_char().is_whitespace() {
            self.advance();
        }
    }

    fn read_number(&mut self) -> Result<Token, LexerError> {
        let start = self.position;
        let mut has_dot = false;

        while self.position < self.input.len() {
            let ch = self.current_char();
            if ch.is_ascii_digit() {
                self.advance();
            } else if ch == '.' && !has_dot && self.peek_char().is_some_and(|c| c.is_ascii_digit())
            {
                has_dot = true;
                self.advance();
            } else {
                break;
            }
        }

        let num_str: String = self.input[start..self.position].iter().collect();

        if has_dot {
            num_str
                .parse::<f64>()
                .map(Token::Float)
                .map_err(|_| LexerError::InvalidNumber(num_str))
        } else {
            num_str
                .parse::<i64>()
                .map(Token::Integer)
                .map_err(|_| LexerError::InvalidNumber(num_str))
        }
    }

    fn read_string(&mut self) -> Result<Token, LexerError> {
        self.advance(); // skip opening quote
        let start = self.position;

        while self.position < self.input.len() && self.current_char() != '\'' {
            self.advance();
        }

        if self.position >= self.input.len() {
            return Err(LexerError::UnterminatedString);
        }

        let string: String = self.input[start..self.position].iter().collect();
        self.advance(); // skip closing quote

        Ok(Token::String(string))
    }

    fn read_identifier_or_keyword(&mut self) -> Result<Token, LexerError> {
        let start = self.position;

        while self.position < self.input.len() {
            let ch = self.current_char();
            if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                self.advance();
            } else {
                break;
            }
        }

        let text: String = self.input[start..self.position].iter().collect();
        let uppercase = text.to_uppercase();

        // Check for multi-word keywords (ORDER BY)
        if uppercase == "ORDER" {
            self.skip_whitespace();
            if self.position < self.input.len() {
                let next_start = self.position;
                let mut next_text = String::new();
                while self.position < self.input.len() {
                    let ch = self.current_char();
                    if ch.is_alphabetic() {
                        next_text.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }
                if next_text.to_uppercase() == "BY" {
                    return Ok(Token::OrderBy);
                }
                // Rollback if not followed by BY
                self.position = next_start;
            }
        }

        // Match keywords
        let token = match uppercase.as_str() {
            "SELECT" => Token::Select,
            "FROM" => Token::From,
            "WHERE" => Token::Where,
            "GROUP" => Token::Group,
            "BY" => Token::By,
            "HAVING" => Token::Having,
            "LIMIT" => Token::Limit,
            "OFFSET" => Token::Offset,
            "JOIN" => Token::Join,
            "INNER" => Token::Inner,
            "LEFT" => Token::Left,
            "RIGHT" => Token::Right,
            "FULL" => Token::Full,
            "ON" => Token::On,
            "AS" => Token::As,
            "AND" => Token::And,
            "OR" => Token::Or,
            "NOT" => Token::Not,
            "LIKE" => Token::Like,
            "IN" => Token::In,
            "BETWEEN" => Token::Between,
            "COUNT" => Token::Count,
            "SUM" => Token::Sum,
            "AVG" => Token::Avg,
            "MIN" => Token::Min,
            "MAX" => Token::Max,
            "ASC" => Token::Asc,
            "DESC" => Token::Desc,
            "TRUE" => Token::Boolean(true),
            "FALSE" => Token::Boolean(false),
            "NULL" => Token::Null,
            _ => Token::Identifier(text),
        };

        Ok(token)
    }
}

/// Lexer errors
#[derive(Debug, Clone, PartialEq)]
pub enum LexerError {
    UnexpectedCharacter(char),
    InvalidNumber(String),
    UnterminatedString,
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexerError::UnexpectedCharacter(ch) => write!(f, "Unexpected character: '{}'", ch),
            LexerError::InvalidNumber(s) => write!(f, "Invalid number: '{}'", s),
            LexerError::UnterminatedString => write!(f, "Unterminated string literal"),
        }
    }
}

impl std::error::Error for LexerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_select() {
        let mut lexer = Lexer::new("SELECT * FROM users");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                Token::Select,
                Token::Asterisk,
                Token::From,
                Token::Identifier("users".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_select_with_where() {
        let mut lexer = Lexer::new("SELECT name FROM users WHERE age > 18");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0], Token::Select);
        assert_eq!(tokens[1], Token::Identifier("name".to_string()));
        assert_eq!(tokens[2], Token::From);
        assert_eq!(tokens[3], Token::Identifier("users".to_string()));
        assert_eq!(tokens[4], Token::Where);
        assert_eq!(tokens[5], Token::Identifier("age".to_string()));
        assert_eq!(tokens[6], Token::Gt);
        assert_eq!(tokens[7], Token::Integer(18));
    }

    #[test]
    fn test_string_literals() {
        let mut lexer = Lexer::new("SELECT * FROM users WHERE name = 'John'");
        let tokens = lexer.tokenize().unwrap();

        assert!(tokens.contains(&Token::String("John".to_string())));
    }

    #[test]
    fn test_order_by() {
        let mut lexer = Lexer::new("SELECT * FROM users ORDER BY name ASC");
        let tokens = lexer.tokenize().unwrap();

        assert!(tokens.contains(&Token::OrderBy));
        assert!(tokens.contains(&Token::Asc));
    }

    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new("= != < <= > >=");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                Token::Eq,
                Token::Ne,
                Token::Lt,
                Token::Le,
                Token::Gt,
                Token::Ge,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 3.5");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![Token::Integer(42), Token::Float(3.5), Token::Eof,]
        );
    }
}
