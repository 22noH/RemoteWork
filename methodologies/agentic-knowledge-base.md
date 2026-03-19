# Agentic Knowledge Base (AKB)

> The canonical reference for AKB — the file-based knowledge protocol for AI agents.

## TL;DR

- **Definition**: A file-based knowledge system where AI uses **search (Grep/Glob)** to find and **contextual links** to navigate
- **Essence**: File-based Lightweight Ontology (Tag = Type, inline links = Edges)
- **Philosophy**: Optimize documents so AI can find them — don't force AI to follow a rigid protocol
- **Structure**: Root (CLAUDE.md) → Hub ({folder}.md) → Node (*.md)
- **Core rules**: 5 writing principles (TL;DR, contextual links, keyword-optimized filenames, atomicity, semantic vs implementation separation)

---

## Architecture

AKB follows a 3-layer hierarchy: **Root → Hub → Node**.

| Layer | Role | Description |
|-------|------|-------------|
| **Root** (CLAUDE.md) | Minimal routing | Auto-injected as system prompt, provides key file paths |
| **Hub** ({folder}.md) | TOC for humans | Folder overview; AI reads selectively |
| **Node** (*.md) | Actual information | What AI searches for via Grep/Glob |

## Writing Principles

1. **TL;DR Required** — 3-5 bullet points with bold keywords for Grep search
2. **Contextual Links** — Place links inline with context, not in isolated lists
3. **Keyword Filenames** — Use descriptive filenames (not notes.md, use market-analysis.md)
4. **Atomicity** — One topic per doc, under 200 lines
5. **Semantic vs Implementation** — AKB holds "why" and relationships; code repo holds specs and configs

## Design Principle

> "Don't try to change AI behavior — optimize documents so AI can find them naturally."

If AI found the information it needed and produced a good answer, that's proof AKB is working.
