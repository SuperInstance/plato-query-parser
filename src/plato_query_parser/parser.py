"""Query parsing with intent classification and keyword extraction."""

import re
from dataclasses import dataclass
from enum import Enum

class Intent(Enum):
    PROCEDURAL = "procedural"
    ANALYTICAL = "analytical"
    CREATIVE = "creative"
    UNKNOWN = "unknown"

@dataclass
class ParsedQuery:
    original: str
    intent: Intent
    keywords: list[str]
    domain: str
    priority: str
    cleaned: str

class QueryParser:
    PROCEDURAL_PATTERNS = ["how to", "how do", "steps to", "way to", "method for", "create", "build", "make", "install", "setup"]
    ANALYTICAL_PATTERNS = ["why", "what is", "explain", "compare", "difference", "analyze", "evaluate", "assess"]
    CREATIVE_PATTERNS = ["imagine", "suggest", "idea", "design", "propose", "brainstorm", "what if"]

    def parse(self, query: str) -> ParsedQuery:
        cleaned = re.sub(r"[^\w\s?]", "", query).strip()
        lower = cleaned.lower()
        intent = self._classify_intent(lower)
        keywords = self._extract_keywords(lower)
        domain = self._detect_domain(keywords)
        priority = self._detect_priority(cleaned)
        return ParsedQuery(original=query, intent=intent, keywords=keywords,
                           domain=domain, priority=priority, cleaned=cleaned)

    def _classify_intent(self, text: str) -> Intent:
        for p in self.PROCEDURAL_PATTERNS:
            if p in text: return Intent.PROCEDURAL
        for p in self.ANALYTICAL_PATTERNS:
            if p in text: return Intent.ANALYTICAL
        for p in self.CREATIVE_PATTERNS:
            if p in text: return Intent.CREATIVE
        return Intent.UNKNOWN

    def _extract_keywords(self, text: str) -> list[str]:
        stop = {"the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
                "have", "has", "had", "do", "does", "did", "will", "would", "could",
                "should", "may", "might", "can", "shall", "to", "of", "in", "for",
                "on", "with", "at", "by", "from", "as", "into", "about", "it", "this",
                "that", "and", "or", "but", "not", "no", "if", "then", "than", "so"}
        return [w for w in text.split() if w not in stop and len(w) > 2]

    def _detect_domain(self, keywords: list[str]) -> str:
        domains = {
            "constraint-theory": {"pythagorean", "snap", "quantization", "holonomy", "manifold", "drift", "ct"},
            "tiles": {"tile", "tiles", "validate", "score", "dedup", "store", "search", "rank"},
            "governance": {"deadband", "priority", "p0", "p1", "p2", "governance", "policy"},
            "forge": {"forge", "training", "adapter", "lora", "fine-tune", "neural", "model"},
            "fleet": {"fleet", "i2i", "bottle", "agent", "vessel", "cocapn", "oracle"},
        }
        kw_set = set(keywords)
        best, best_overlap = "unknown", 0
        for domain, domain_kw in domains.items():
            overlap = len(kw_set & domain_kw)
            if overlap > best_overlap:
                best, best_overlap = domain, overlap
        return best

    def _detect_priority(self, text: str) -> str:
        if any(k in text.lower() for k in ["urgent", "critical", "error", "fail", "p0"]):
            return "P0"
        if any(k in text.lower() for k in ["important", "warning", "p1"]):
            return "P1"
        return "P2"
