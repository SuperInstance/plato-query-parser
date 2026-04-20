"""Query parser — natural language to structured queries with tokenization and AST."""
import re
from dataclasses import dataclass, field
from typing import Optional
from enum import Enum

class TokenType(Enum):
    WORD = "word"
    QUOTED = "quoted"
    OPERATOR = "operator"
    COMPARISON = "comparison"
    LOGIC = "logic"
    WILDCARD = "wildcard"
    NEGATION = "negation"
    TAG = "tag"
    PAREN_OPEN = "paren_open"
    PAREN_CLOSE = "paren_close"

@dataclass
class Token:
    type: TokenType
    value: str
    position: int = 0

@dataclass
class QueryClause:
    field: str = ""
    operator: str = "="
    value: str = ""
    negated: bool = False
    logic: str = "AND"

@dataclass
class ParsedQuery:
    clauses: list[QueryClause] = field(default_factory=list)
    raw: str = ""
    keywords: list[str] = field(default_factory=list)
    tags: list[str] = field(default_factory=list)
    domains: list[str] = field(default_factory=list)
    sort_by: str = ""
    sort_order: str = "desc"
    limit: int = 20
    page: int = 1

class QueryParser:
    OPERATORS = {"=", "!=", ">", "<", ">=", "<=", "~", "contains", "starts", "ends"}
    LOGIC_OPS = {"and", "or", "not"}
    SORT_OPS = {"sort:", "order:", "limit:", "page:"}
    FIELD_ALIASES = {"domain": "domain", "tag": "tags", "room": "room",
                     "from": "created_after", "before": "created_before",
                     "confidence": "confidence", "author": "author"}

    def parse(self, query: str) -> ParsedQuery:
        if not query or not query.strip():
            return ParsedQuery()
        tokens = self._tokenize(query)
        pq = ParsedQuery(raw=query)
        pq.clauses = self._build_clauses(tokens)
        pq.keywords = [t.value for t in tokens if t.type == TokenType.WORD]
        pq.tags = [c.value for c in pq.clauses if c.field == "tags"]
        pq.domains = [c.value for c in pq.clauses if c.field == "domain"]
        # Extract meta commands
        meta_tokens = [t for t in tokens if t.type == TokenType.WORD and t.value.startswith("sort:")]
        if meta_tokens:
            pq.sort_by = meta_tokens[0].value.split(":", 1)[1]
        limit_tokens = [t for t in tokens if t.type == TokenType.WORD and t.value.startswith("limit:")]
        if limit_tokens:
            try: pq.limit = int(limit_tokens[0].value.split(":", 1)[1])
            except: pass
        return pq

    def _tokenize(self, query: str) -> list[Token]:
        tokens = []
        i = 0
        while i < len(query):
            c = query[i]
            if c.isspace():
                i += 1
                continue
            if c == '"':
                j = query.index('"', i + 1) if '"' in query[i+1:] else len(query)
                tokens.append(Token(TokenType.QUOTED, query[i+1:j], i))
                i = j + 1
            elif c == '#':
                j = i + 1
                while j < len(query) and (query[j].isalnum() or query[j] in "-_"):
                    j += 1
                tokens.append(Token(TokenType.TAG, query[i+1:j], i))
                i = j
            elif c == '(':
                tokens.append(Token(TokenType.PAREN_OPEN, "(", i))
                i += 1
            elif c == ')':
                tokens.append(Token(TokenType.PAREN_CLOSE, ")", i))
                i += 1
            elif c == '-' and i + 1 < len(query) and query[i+1].isalpha():
                tokens.append(Token(TokenType.NEGATION, "-", i))
                i += 1
            elif c == '*':
                tokens.append(Token(TokenType.WILDCARD, "*", i))
                i += 1
            elif c == ':' and tokens and tokens[-1].type == TokenType.WORD:
                tokens[-1].value += ":"
                tokens[-1].type = TokenType.OPERATOR
                i += 1
            elif c in "><!~" and i + 1 < len(query) and query[i+1] == "=":
                tokens.append(Token(TokenType.COMPARISON, query[i:i+2], i))
                i += 2
            elif c in "><=":
                tokens.append(Token(TokenType.COMPARISON, c, i))
                i += 1
            elif query[i:].lower().startswith(tuple(f" {op} " for op in self.LOGIC_OPS)):
                for op in self.LOGIC_OPS:
                    if query[i:].lower().startswith(f" {op} "):
                        tokens.append(Token(TokenType.LOGIC, op.upper(), i))
                        i += len(op) + 2
                        break
            else:
                j = i
                while j < len(query) and not query[j].isspace() and query[j] not in '()":*!~<>=':
                    j += 1
                word = query[i:j]
                if word.lower() in self.LOGIC_OPS:
                    tokens.append(Token(TokenType.LOGIC, word.upper(), i))
                elif word.lower() in ("and", "or"):
                    tokens.append(Token(TokenType.LOGIC, word.upper(), i))
                else:
                    tokens.append(Token(TokenType.WORD, word, i))
                i = j
        return tokens

    def _build_clauses(self, tokens: list[Token]) -> list[QueryClause]:
        clauses = []
        i = 0
        while i < len(tokens):
            t = tokens[i]
            # field:value pattern
            if t.type == TokenType.OPERATOR and i + 1 < len(tokens):
                field_name = t.value.rstrip(":")
                field_name = self.FIELD_ALIASES.get(field_name, field_name)
                next_t = tokens[i + 1]
                value = next_t.value
                op = "="
                clauses.append(QueryClause(field=field_name, operator=op, value=value))
                i += 2
            # #tag pattern
            elif t.type == TokenType.TAG:
                clauses.append(QueryClause(field="tags", operator="=", value=t.value))
                i += 1
            # negation
            elif t.type == TokenType.NEGATION and i + 1 < len(tokens):
                next_t = tokens[i + 1]
                if next_t.type == TokenType.TAG:
                    clauses.append(QueryClause(field="tags", operator="!=", value=next_t.value, negated=True))
                    i += 2
                else:
                    clauses.append(QueryClause(field="content", operator="!=", value=next_t.value, negated=True))
                    i += 2
            # comparison
            elif t.type == TokenType.COMPARISON and i >= 1:
                prev = tokens[i - 1]
                if i + 1 < len(tokens):
                    clauses.append(QueryClause(field=prev.value, operator=t.value,
                                              value=tokens[i + 1].value))
                    i += 2
                else:
                    i += 1
            # meta commands (sort:, limit:, etc.)
            elif t.type == TokenType.WORD and ":" in t.value and t.value.startswith(tuple(self.SORT_OPS)):
                i += 1
            else:
                i += 1
        return clauses

    def to_sql_like(self, query: str) -> str:
        pq = self.parse(query)
        parts = []
        for c in pq.clauses:
            if c.field == "content":
                parts.append(f"content LIKE '%{c.value}%'")
            elif c.operator == "!=":
                parts.append(f"{c.field} != '{c.value}'")
            elif c.operator == "~":
                parts.append(f"{c.field} LIKE '%{c.value}%'")
            else:
                parts.append(f"{c.field} = '{c.value}'")
        where = " AND ".join(parts) if parts else "1=1"
        order = f"ORDER BY {pq.sort_by} {pq.sort_order.upper()}" if pq.sort_by else ""
        return f"SELECT * FROM tiles WHERE {where} {order} LIMIT {pq.limit}"

    @property
    def stats(self) -> dict:
        return {"operators": len(self.OPERATORS), "field_aliases": len(self.FIELD_ALIASES)}
