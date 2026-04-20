use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Word,
    Quoted,
    Operator,
    Comparison,
    Logic,
    Wildcard,
    Negation,
    Tag,
    ParenOpen,
    ParenClose,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub value: String,
    pub position: usize,
}

#[derive(Debug, Clone)]
pub struct QueryClause {
    pub field: String,
    pub operator: String,
    pub value: String,
    pub negated: bool,
    pub logic: String,
}

impl Default for QueryClause {
    fn default() -> Self {
        QueryClause {
            field: String::new(),
            operator: "=".to_string(),
            value: String::new(),
            negated: false,
            logic: "AND".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedQuery {
    pub clauses: Vec<QueryClause>,
    pub raw: String,
    pub keywords: Vec<String>,
    pub tags: Vec<String>,
    pub domains: Vec<String>,
    pub sort_by: String,
    pub sort_order: String,
    pub limit: i32,
    pub page: i32,
}

impl Default for ParsedQuery {
    fn default() -> Self {
        ParsedQuery {
            clauses: Vec::new(),
            raw: String::new(),
            keywords: Vec::new(),
            tags: Vec::new(),
            domains: Vec::new(),
            sort_by: String::new(),
            sort_order: "desc".to_string(),
            limit: 20,
            page: 1,
        }
    }
}

pub struct QueryParser {
    operators: Vec<String>,
    logic_ops: Vec<String>,
    sort_ops: Vec<String>,
    field_aliases: HashMap<String, String>,
}

impl Default for QueryParser {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryParser {
    pub fn new() -> Self {
        let mut field_aliases = HashMap::new();
        field_aliases.insert("domain".to_string(), "domain".to_string());
        field_aliases.insert("tag".to_string(), "tags".to_string());
        field_aliases.insert("room".to_string(), "room".to_string());
        field_aliases.insert("from".to_string(), "created_after".to_string());
        field_aliases.insert("before".to_string(), "created_before".to_string());
        field_aliases.insert("confidence".to_string(), "confidence".to_string());
        field_aliases.insert("author".to_string(), "author".to_string());

        QueryParser {
            operators: vec![
                "=".to_string(),
                "!=".to_string(),
                ">".to_string(),
                "<".to_string(),
                ">=".to_string(),
                "<=".to_string(),
                "~".to_string(),
                "contains".to_string(),
                "starts".to_string(),
                "ends".to_string(),
            ],
            logic_ops: vec!["and".to_string(), "or".to_string(), "not".to_string()],
            sort_ops: vec![
                "sort:".to_string(),
                "order:".to_string(),
                "limit:".to_string(),
                "page:".to_string(),
            ],
            field_aliases,
        }
    }

    pub fn parse(&self, query: &str) -> ParsedQuery {
        if query.trim().is_empty() {
            return ParsedQuery::default();
        }
        let tokens = self.tokenize(query);
        let mut pq = ParsedQuery {
            raw: query.to_string(),
            ..ParsedQuery::default()
        };
        pq.clauses = self.build_clauses(&tokens);
        pq.keywords = tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Word)
            .map(|t| t.value.clone())
            .collect();
        pq.tags = pq
            .clauses
            .iter()
            .filter(|c| c.field == "tags")
            .map(|c| c.value.clone())
            .collect();
        pq.domains = pq
            .clauses
            .iter()
            .filter(|c| c.field == "domain")
            .map(|c| c.value.clone())
            .collect();

        // Extract meta commands
        let meta_tokens: Vec<&Token> = tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Word && t.value.starts_with("sort:"))
            .collect();
        if let Some(mt) = meta_tokens.first() {
            if let Some((_, v)) = mt.value.split_once(':') {
                pq.sort_by = v.to_string();
            }
        }

        let limit_tokens: Vec<&Token> = tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Word && t.value.starts_with("limit:"))
            .collect();
        if let Some(lt) = limit_tokens.first() {
            if let Some((_, v)) = lt.value.split_once(':') {
                if let Ok(n) = v.parse::<i32>() {
                    pq.limit = n;
                }
            }
        }

        pq
    }

    fn tokenize(&self, query: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut i = 0;
        let chars: Vec<char> = query.chars().collect();

        while i < chars.len() {
            let c = chars[i];
            if c.is_whitespace() {
                i += 1;
                continue;
            }

            if c == '"' {
                let mut j = i + 1;
                while j < chars.len() && chars[j] != '"' {
                    j += 1;
                }
                let value: String = chars[(i + 1)..j].iter().collect();
                tokens.push(Token {
                    token_type: TokenType::Quoted,
                    value,
                    position: i,
                });
                i = j + 1;
            } else if c == '#' {
                let mut j = i + 1;
                while j < chars.len()
                    && (chars[j].is_alphanumeric() || chars[j] == '-' || chars[j] == '_')
                {
                    j += 1;
                }
                let value: String = chars[(i + 1)..j].iter().collect();
                tokens.push(Token {
                    token_type: TokenType::Tag,
                    value,
                    position: i,
                });
                i = j;
            } else if c == '(' {
                tokens.push(Token {
                    token_type: TokenType::ParenOpen,
                    value: "(".to_string(),
                    position: i,
                });
                i += 1;
            } else if c == ')' {
                tokens.push(Token {
                    token_type: TokenType::ParenClose,
                    value: ")".to_string(),
                    position: i,
                });
                i += 1;
            } else if c == '-' && i + 1 < chars.len() && chars[i + 1].is_alphabetic() {
                tokens.push(Token {
                    token_type: TokenType::Negation,
                    value: "-".to_string(),
                    position: i,
                });
                i += 1;
            } else if c == '*' {
                tokens.push(Token {
                    token_type: TokenType::Wildcard,
                    value: "*".to_string(),
                    position: i,
                });
                i += 1;
            } else if c == ':' && !tokens.is_empty() && tokens.last().unwrap().token_type == TokenType::Word {
                if let Some(last) = tokens.last_mut() {
                    last.value.push(':');
                    last.token_type = TokenType::Operator;
                }
                i += 1;
            } else if "><!~".contains(c) && i + 1 < chars.len() && chars[i + 1] == '=' {
                let value: String = chars[i..(i + 2)].iter().collect();
                tokens.push(Token {
                    token_type: TokenType::Comparison,
                    value,
                    position: i,
                });
                i += 2;
            } else if c == '>' || c == '<' || c == '=' {
                tokens.push(Token {
                    token_type: TokenType::Comparison,
                    value: c.to_string(),
                    position: i,
                });
                i += 1;
            } else {
                // Check for logic ops
                let remaining: String = chars[i..].iter().collect();
                let lower_remaining = remaining.to_lowercase();
                let mut found_logic = false;
                for op in &self.logic_ops {
                    let pattern = format!(" {} ", op);
                    if lower_remaining.starts_with(&pattern) {
                        tokens.push(Token {
                            token_type: TokenType::Logic,
                            value: op.to_uppercase(),
                            position: i,
                        });
                        i += op.len() + 2;
                        found_logic = true;
                        break;
                    }
                }

                if !found_logic {
                    let mut j = i;
                    while j < chars.len()
                        && !chars[j].is_whitespace()
                        && !"()\":*!~<>=".contains(chars[j])
                    {
                        j += 1;
                    }
                    let word: String = chars[i..j].iter().collect();
                    let lower_word = word.to_lowercase();
                    if self.logic_ops.contains(&lower_word) {
                        tokens.push(Token {
                            token_type: TokenType::Logic,
                            value: word.to_uppercase(),
                            position: i,
                        });
                    } else {
                        tokens.push(Token {
                            token_type: TokenType::Word,
                            value: word,
                            position: i,
                        });
                    }
                    i = j;
                }
            }
        }

        tokens
    }

    fn build_clauses(&self, tokens: &[Token]) -> Vec<QueryClause> {
        let mut clauses = Vec::new();
        let mut i = 0;

        while i < tokens.len() {
            let t = &tokens[i];

            if t.token_type == TokenType::Operator && i + 1 < tokens.len() {
                let field_name = t.value.trim_end_matches(':');
                let field_name = self.field_aliases.get(field_name).map(|s| s.as_str()).unwrap_or(field_name);
                let next_t = &tokens[i + 1];
                clauses.push(QueryClause {
                    field: field_name.to_string(),
                    operator: "=".to_string(),
                    value: next_t.value.clone(),
                    ..QueryClause::default()
                });
                i += 2;
            } else if t.token_type == TokenType::Tag {
                clauses.push(QueryClause {
                    field: "tags".to_string(),
                    operator: "=".to_string(),
                    value: t.value.clone(),
                    ..QueryClause::default()
                });
                i += 1;
            } else if t.token_type == TokenType::Negation && i + 1 < tokens.len() {
                let next_t = &tokens[i + 1];
                if next_t.token_type == TokenType::Tag {
                    clauses.push(QueryClause {
                        field: "tags".to_string(),
                        operator: "!=".to_string(),
                        value: next_t.value.clone(),
                        negated: true,
                        ..QueryClause::default()
                    });
                } else {
                    clauses.push(QueryClause {
                        field: "content".to_string(),
                        operator: "!=".to_string(),
                        value: next_t.value.clone(),
                        negated: true,
                        ..QueryClause::default()
                    });
                }
                i += 2;
            } else if t.token_type == TokenType::Comparison && i >= 1 && i + 1 < tokens.len() {
                let prev = &tokens[i - 1];
                clauses.push(QueryClause {
                    field: prev.value.clone(),
                    operator: t.value.clone(),
                    value: tokens[i + 1].value.clone(),
                    ..QueryClause::default()
                });
                i += 2;
            } else if t.token_type == TokenType::Word && t.value.contains(':') {
                let starts_with_sort_op = self.sort_ops.iter().any(|op| t.value.starts_with(op));
                if starts_with_sort_op {
                    i += 1;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        clauses
    }

    pub fn to_sql_like(&self, query: &str) -> String {
        let pq = self.parse(query);
        let mut parts = Vec::new();
        for c in &pq.clauses {
            if c.field == "content" {
                parts.push(format!("content LIKE '%{}%'", c.value));
            } else if c.operator == "!=" {
                parts.push(format!("{} != '{}'", c.field, c.value));
            } else if c.operator == "~" {
                parts.push(format!("{} LIKE '%{}%'", c.field, c.value));
            } else {
                parts.push(format!("{} = '{}'", c.field, c.value));
            }
        }
        let where_clause = if parts.is_empty() {
            "1=1".to_string()
        } else {
            parts.join(" AND ")
        };
        let order = if !pq.sort_by.is_empty() {
            format!("ORDER BY {} {}", pq.sort_by, pq.sort_order.to_uppercase())
        } else {
            String::new()
        };
        format!(
            "SELECT * FROM tiles WHERE {} {} LIMIT {}",
            where_clause, order, pq.limit
        )
    }

    pub fn stats(&self) -> HashMap<String, usize> {
        let mut map = HashMap::new();
        map.insert("operators".to_string(), self.operators.len());
        map.insert("field_aliases".to_string(), self.field_aliases.len());
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parse() {
        let parser = QueryParser::new();
        let pq = parser.parse("hello world");
        assert_eq!(pq.keywords, vec!["hello", "world"]);
    }

    #[test]
    fn test_tag_parse() {
        let parser = QueryParser::new();
        let pq = parser.parse("#rust");
        assert_eq!(pq.tags, vec!["rust"]);
    }

    #[test]
    fn test_sql_like() {
        let parser = QueryParser::new();
        let sql = parser.to_sql_like("domain:test");
        assert!(sql.contains("domain = 'test'"));
    }
}
