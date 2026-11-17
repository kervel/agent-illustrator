# Design: connections (and other statements) from data

Status: approved. **S1 (P1) released in v0.2.49; S2 (P2), S3 (P3) and S5 (P5) implemented**; S4 and S6 not started.
Maintainer decisions: keywords may name fields (open question 1: yes); a missing field is an
error, there are no defaults (open question 4). Motivating example: `examples/sequence-diagram.ail`.

S1 as built (differences from §2 P1 noted):
- `joined_path` keeps dots; `parser::join_constraint_parts` joins them at the end of `parse`
  (skipping `each` bodies) and again on every expanded copy (`each.rs`).
- Shared helpers in `src/template/names.rs` (`head_of`, `splice_dotted`, `splice_path`), also
  used by the element-reference parameters in `resolver.rs`.
- Besides the listed name positions, `place` subjects/anchors and **any** modifier value of the
  form `n.part` are spliced (a bare `n` was already replaced). Items written as keywords
  (`end`, `center`) count as names.
- The shadowing check covers the loop variable and the index, against names declared in the
  same scope (the document, or the template body). It does not see names from outside a template.
- Example: `examples/fan-out.ail`. Docs: skill.md (`each` + Reach For row), grammar.md,
  examples.md, `--doc fan-out`.

S2 as built (differences from §2 P2 noted):
- Keyword field names come from a token pass (`field_words` in `parser/grammar.rs`), not from a
  `word` parser. A keyword becomes a plain word as a record key (after the `{` or `,` of a `{`
  that opens a list item, before `:`), and after a `.`. The exceptions after a dot are the words
  that already mean something there: edges, centres, `line`, `row`.
- A dotted list or record value may end in an edge (`from: failed.left`, `via: [a.top]`).
- `text m.label` works (the `text NAME` shorthand takes a dotted name).
- Problems are collected over all items and reported together. A field that no item has gives
  "no item has a field `lable` (item 1 has: ...); did you mean `label`?". A field missing from
  only some items gives "items 2 and 3 have no field `label`, which the other items have".
- A whole record is an error in every position, including `constrain t.width`, where `width` is
  a property, not a field.
- `m.f.part` in a value position whose field is not a name ("`m.f` is the string "x", which
  has no part `part`") is an error. In S1 such a value was left alone.
- The class template still uses its four status lists. Moving it to one record list needs
  `style: a.st`, so it waits for named modifier sets (a).
- Example: `examples/state-machine.ail`. Docs: skill.md (`each` + Reach For row), grammar.md,
  examples.md, `--doc records`.

S3 as built (differences from §2 P3 noted):
- A second token pass, `indexed_names` (`parser/grammar.rs`), runs after `indexed_parts`, so
  `line[4]`, `row[2]`, `cell[r][c]` keep their sugar; `line`/`lines`/`row`/`rows`/`cell`/`cells`
  never take the generic `_k`. A literal whole number becomes `p_6` at once.
- **Bound or not is decided at parse time, not after expansion.** The pass tracks the loop
  variables of the enclosing `each` bodies (by braces). A word or computed index whose head
  is bound becomes `p_{i+1}` / `p_{m.row}` / `p_{m}`; an unbound word is a literal key
  (`p[query]` = `p_query`); an unbound `i+1` or `m.row` is a parse error with a caret
  ("`i` is not bound here; an index like i+1 only works inside `each m, i in ...`"). A
  fraction (`p[2.5]`) and a negative literal are parse errors too. So the resolver pass that
  rewrites leftovers is not needed; `locate_error` keeps a backstop for a `_{` that reaches
  layout.
- `p_{m.row}` has a dot inside one name, so `each.rs` fills indices (`fill_index`) in every
  name position **before** the name is split at its dots (path segments, connection element
  and anchor, contains/midpoint operands, place subject/anchor, modifier and template
  argument values, declared shape names). An index whose variable is not bound by this block
  is left for the inner block. The value must be a whole number ≥ 0 or a word; a fraction,
  negative result (`p[i-1]` on item 1), quoted string or dotted name is reported per item
  with the rest of the block's problems.
- The `p_0` hint and an out-of-range hint share one helper (`layout::copy_hint`): `find_similar`
  returns the first and last copy of a numbered family, and the message says "`api.p` has
  copies 1 to 3 (api.p_1 … api.p_3); there is no copy 7" (for `_0`: "... write api.p[i+1],
  not api.p[i]"). A connection end `api.p[7]` used to fail as an *invalid anchor*; when
  `api` has numbered `p_k` parts it now gives the same undefined-name message with a caret.
- Template parameters are not indices: `p[n]` with a parameter `n` is the literal `p_n`.
- Example: `examples/pipeline.ail` (stages from one `each`, hand-offs `stage[k] ->
  stage[k+1]` from another, single stages by number). The sequence example is unchanged.
  Docs: skill.md (`each` + Reach For row), grammar.md, examples.md, `--doc index`/`pipeline`.

S5 as built (differences from §2 P5 noted):
- The trigger is implicit, as recommended: records that all have an `id` field (open question
  3). `item_keys` in `each.rs` gives the suffixes; the rename map uses them instead of `k+1`.
- Also new: a declared name whose index this block binds (`rect lane[l]`, `rect p[m.id]`) keeps
  that name and gets no suffix (it used to become `lane_cust_1`). Two items that make the
  same declared name are an error ("items 1 and 2 both make an element called `r_a`").
- An id may be a whole number as well as a word.
- The numeric-index hint on a keyed family (`api.p[3]`) is not specialised yet; the existing
  undefined-name message lists the similar names.
- Example: `examples/cross-functional.ail` (lanes `lanes_cust`, `lanes_am`, reached as `lanes[am]`).
  Docs: skill.md (`each`), grammar.md, examples.md, `--doc keyed`.

## 1. The problem, measured against today's code

The 9 messages in the sequence example are hand-written lines whose endpoints, labels,
frames and activation bars all hard-code row numbers (`api.p_6 -> db.p_6 [label: "5: query"]`).
Inserting one message means renumbering ~25 places. The row of a message is its position in a
list, but the language cannot say "item k of a list makes a connection between X_k and Y_k".

What `each` can and cannot do today (checked against `src/template/each.rs` and by running
`target/debug/agent-illustrator` on small probes):

| Probe | Result today | Cause |
|---|---|---|
| `each n in [a, b] { n -> hub }` | `undefined identifier 'n'` | `subst_stmt` substitutes modifiers, text and instance args only; connection endpoints, `constrain`, `contains`, `midpoint` and anchor positions fall through `other => other` |
| `each n in [a, b] { constrain n.width = 50 }` | `undefined identifier 'n'` | same |
| `each m in [{src: a, lbl: "x"}] { ... }` | `item 1 is a record; items must be strings or numbers` | `expand_block` accepts only scalar items. The parser already accepts record lists (`value_list` includes `record`). |
| `[label: m.lbl]` | parses (`dotted_atom`) but is never resolved | there is no field access |
| `[label: m.label]`, `{from: a}` | parse error | `label` is a keyword token (a `dotted_atom` segment must be an identifier); `from` is reserved |
| `a.p[3] -> hub` | parse error | `indexed_parts` (grammar.rs:79) desugars `[N]` only after `.line`/`.row`/`.cell`, to `line3`/`row3`, with no `_` |
| `a -> a` | a meaningless 26 px vertical stub | there is no self-loop route (relevant to §6) |

Other facts the design depends on:
- `contains` and `midpoint` operands go through `joined_path`, which turns `a.p_6` into `a_p_6`
  **at parse time** (grammar.rs:1129, 1295). A `m.src.p[...]` there would be flattened to
  `m_src_p...` before `each` could split off `m`.
- In records and modifiers a bare word becomes an `Identifier`, except a fixed list (`none`,
  `end`, `center`, `dashed`, colour names, ...) that becomes `Keyword` (grammar.rs:753).
- Top-level `each` gets an empty parameter map (resolver.rs:108), so it only iterates literal
  lists. There is no top-level named data. Inside a template, `life x [ticks: msgs]` **does**
  receive the outer template's list parameter (resolver.rs:893 substitutes `Identifier` args).
- Each item's copies are named `NAME_<k>` with k 1-based, and `i` is 0-based. Renaming happens
  **before** substitution, so a name that arrives through data is never suffixed. Keep that
  invariant: data refers to things outside the body.

## 2. Primitives

There are six. P1–P3 are the core and are useful on their own. P4–P6 make the sequence example
fully insertion-stable. None of them names a diagram type.

### P1. Loop variables work in every name position

**Syntax:** nothing new. `each c in [c1, c2, c3] { srv.right -> c.left }`,
`constrain c.width = 80`, `constrain bg contains c`, `via: [c]`, `overlaps: [c]`,
`anchor a [position: c.top]`.

**Semantics:** wherever an element name goes, a loop variable whose value is a name (an
`Identifier`, or a `Keyword` such as `end`) is replaced by that name. That covers connection
endpoints (`AnchorReference.element`), `ElementPath` heads in `constrain`, `contains` /
`midpoint` operands, `Constraint.subject`, anchor positions, and identifier lists in
`via`/`through`/`clip`/`drawn`/`overlaps` (the same keys `rn_mods` already knows about). A
dotted value (`c: board.led`) splices its tail into the path (`c.top` becomes `board.led.top`).

**Parser cost:** `contains`/`midpoint` must keep the dots until `each` has run. Make
`joined_path` produce `a.p_6` (dots kept), and join to `a_p_6` in a **separate pass over the
whole AST** right after `each` expansion. It cannot happen inside `expand_each`, because that
only recurses into `Layout`/`Group`/`Each`. `constrain` also lives in `NamedLayout`
(`layout beside { ... }`), in `Label`, and in template bodies. Alternatively, store
`ElementPath` in `ConstraintExpr::Contains`/`Midpoint` and join where they are consumed.
No lexer change.

**Edge cases and errors:**
- The loop variable has the same name as an element in scope, **and the body uses it in a name
  position**. Today the variable silently wins. Make that case an error:
  `each n in ...: 'n' is also an element; rename the loop variable (each it in ...)`.
  Keeping the check to name positions leaves existing files (such as the `each a in attrs`
  bodies) valid.
- A value that is not a name in a name position (`each n in ["a"] { n -> hub }`) gives:
  `n -> hub: n is the string "a" (item 1); a name goes without quotes: each n in [a, b]`.
- A substituted name that does not exist is reported by the existing "undefined identifier"
  error. The substituted `Identifier` gets the **span of the list item**, so the caret lands on
  `[a, bb]`, where the typo is.

**Cost:** each.rs only: about 60 lines in `subst_stmt` plus a `subst_name` helper. Share that
helper with the concurrent element-reference template parameters (b), which need the same
"value that names an element" substitution in the same positions.

### P2. `each` over records, with field access

**Syntax:**
```
each t in [{src: idle, dst: run, on: "start"}, {src: run, dst: idle, on: "stop"}] {
  t.src -> t.dst [label: t.on]          // or label: "{t.on}"
}
```
**Semantics:**
- An item may be a record. `VAR.FIELD` is the field's value in a value position, in a name
  position (with P1 rules: `t.src.right` becomes `idle.right`) and in strings (`"{t.on}"`,
  `"{i+1}: {t.on}"`).
- A bare `VAR` that is a record is an error in any position: `t is a record; pick a field:
  t.src, t.dst, t.on`.
- Field values are atoms, as in records today: strings, numbers, colours, names and dotted
  names.

**Parser cost:**
- Record keys and the segment after a loop variable must accept keyword tokens (`label`,
  `text`, `from`, `to`, `top`, ...). Fields live in their own namespace, so `{from: cl}` and
  `m.from` should be legal. Add a `word` parser (identifier | any keyword token → its text) and
  use it in `record_key`, in `dotted_atom` tails and in `anchor_name`.
- Single-value modifiers already accept `dotted_atom`.

**each.rs cost:**
- Accept `Record` items.
- `subst_value` and `interpolate` learn `VAR.FIELD`. The interpolation regex becomes
  `{name(.field)?([+-]int)?}`, and `{i+1}` keeps working.

**Errors:**
- `each m in msgs: item 3 has no field 'lable' (m.lable, line 41); its fields: id, from, to,
  kind, label. Did you mean 'label'?` The caret is at the use, and the item number is given.
- If a used field is missing from only some items, the message names the items:
  `items 4, 7 have no 'label', which the body uses (line 41)`.
- `label: m` (a whole record) gives the error above.
- Unknown field names are caught at expansion time, before layout, so the agent gets them all
  at once (collect, don't stop at the first).

### P3. Indexed names: `x.p[k]`

**Syntax:**
- `api.p[6]` is the same as `api.p_6`, everywhere a name goes.
- Inside `each`, the index may be computed: `p[i+1]`, `p[i-1]`, `p[m.row]` (a number field),
  `p[m.id]` (a key, see P5).

**Semantics:** `NAME[k]` names the k-th copy that an `each` made of `NAME` (1-based, as the
names are). The reserved built-in parts keep their current sugar (`t.row[2]`=`row2`,
`c.line[4]`, `t.cell[r][c]`). For every other word, `[k]` means `_k`.

**Parser cost:** generalise `indexed_parts` (token pass):
- An identifier **immediately** followed by `[` (no whitespace, checked by spans), then one of
  `int` | `word` | `word.word` | `word ± int`, then `]`, becomes a single `Ident` token:
  `p_6`, `p_{i+1}`, `p_{m.id}`, `p_{query}`.
- The adjacency requirement keeps `rect a [fill: red]` and `x -> y [auto]` unchanged. A
  modifier block whose content has a `:` never matches anyway.
- `each` interpolates `{...}` in identifiers with the same function as for strings.

**Resolver cost:** after all `each` expansion, a leftover `p_{word}` becomes `p_word` (a
literal key, P5). A leftover `{i+1}` or `{m.f}` is an error:
`api.p[i+1]: 'i' is not bound here; an index like i+1 only works inside 'each m, i in ...'`.

**Off-by-one trap:** `i` is 0-based, names are 1-based. `p[i]` therefore produces `p_0` on the
first item, which never exists. Special-case the undefined-name error for `_0`:
`p_0 does not exist: copies are numbered from 1 and i counts from 0; write p[i+1]`.
Recommend keys (P5) in the docs, so no arithmetic is needed at all.

**Why `[k]` and not `p_{i+1}` at the surface:** `{` is a block token, so `ticks{` vs `p_{`
would be lexer-ambiguous. `[k]` also matches the existing `row[2]`/`line[4]` sugar, and it
reads as indexing.

### P4. Named data: `let NAME = VALUE`

**Syntax:** top level only: `let msgs = [{...}, ...]`.

**Semantics:**
- `NAME` can be used as an `each` source and as a template argument value
  (`life cl [ticks: msgs]`). That is how one list drives several blocks and instances.
- Lets are visible in the whole file (including templates declared in it, as the default of a
  parameter). Order does not matter. Redefinition is an error.
- Using a let as a style value (`fill: msgs`) is an error:
  `msgs is data (a list of 9 records); use it in 'each m in msgs' or as a template argument`.

**Why it is needed:** today the only way to share a list is to wrap the whole diagram in a
template with a `msgs: []` parameter. Every outside reference then gains a prefix (`s.api.p_6`),
and the diagram is no longer a plain top-level document.

**Cost:**
- lexer: one keyword (`let`). This is a new reserved word, so update `--doc reserved`.
- grammar: one statement.
- resolver: collect the lets, pass them as `lists` to the top-level `expand_each`
  (resolver.rs:108), and substitute them in top-level instance arguments, the way resolver.rs:893
  already does for nested instances. About 80 lines.
- The `each` unknown-list error (each.rs) lists lets too.

### P5. Keyed copies: an item's `id` names its copies

**Semantics:**
- If the items are records that all have an `id:` field, the copies are named by it instead of
  by number: in `each t in ticks { rect p }` with `ticks: msgs`, the rows are `p_login`,
  `p_verify`, ... rather than `p_1`, `p_2`.
- With P3, `api.p[login]` names a row by meaning, so inserting a message renumbers nothing.
- `i` stays the numeric position (labels still say "{i+1}").

**each.rs cost:** about 20 lines. The rename map uses the key instead of `k+1`.

**Errors:**
- `id` must be a word: `item 3: id "x y" must be a name (letters, digits, _)`.
- `id` must be unique: `items 2 and 5 both have id 'ok'`.
- It is all or none: `item 4 has no id; give every item an id, or none`.
- `api.p[3]` on a keyed family: the undefined-name hint lists the family:
  `api has p_login, p_verify, ...: name it p[login]`.

**Alternative syntax** (open question 3): an explicit `each t in ticks by id`. The implicit rule
was chosen because a generic template (`life`) cannot know whether its caller passes keyed
records or numbers, and an explicit `by id` would make the template reject plain numbers.

### P6. Filtering: `each m in msgs where COND`

**Syntax:** `where m.f == VALUE`, `!=`, and `m.f == m.g` (field vs field). Conditions are
joined with `and`. That is all; there is no general expression language.

**Semantics:**
- The body runs only for matching items.
- `i` and the copy names keep the item's position in the **source list**, so numbering stays
  aligned with every other `each` over the same list.

**Why it is needed:** two items of one list sometimes need different statements, not just
different styles. In the sequence diagram, a self-message needs waypoints and different
endpoints. Splitting it into a second list breaks "one list = one row order". It is general:
- state-machine self-transitions (`where t.src == t.dst`);
- milestones vs tasks in a timeline;
- optional legend rows.

**Cost:**
- lexer: `==` and `!=` tokens (only `=` exists today), or the words `is` / `is not`.
- grammar: an optional `where` clause on `each_block`. `where` and `and` are words only in that
  position, like `in`.
- each.rs: a predicate per item, about 40 lines.

**Errors:** unknown field (as in P2). Comparing with a record or list is an error.

### Dependencies on concurrent work
- **(a) Named modifier sets:** `kind: reply` selects a look through `[style: m.kind]`.
  Without (a), `kind` can only feed individual modifiers (`to_end: m.end`, `stroke_dasharray:
  m.dash`), which means more fields per item. P2's only requirement is that the style
  reference is an ordinary identifier value, so substitution yields `style: reply`. Whatever
  syntax (a) lands with works, as long as it takes a plain name. This doc writes `style:`,
  but `style:` already exists as an `accent` option (`style: auto|underline|ring|...`), so (a)
  may need another key.
- **(b) Element-reference template parameters:** share P1's `subst_name` helper. They compose:
  an `each` body may instantiate a template with `[a: m.from, b: m.to]`.
- **(c) Implicit overlap groups:** these shrink the long `overlaps:` lists on frames. They are
  orthogonal to this design.

## 3. Domain-agnostic check

Every primitive is about lists, names and records. None knows what a message, row or class is.
Uses beyond sequence diagrams:

1. **Fan-out / network (P1 only).** The CLAUDE.md success prompt, "a server sending data to
   three clients":
   `each c in [c1, c2, c3] { srv.right -> c.left [label: "data"] }`. This is impossible today.
2. **State machine from a transition table (P1, P2, P4, P6):**
   ```
   let tr = [{src: idle, dst: running, on: "start"}, {src: running, dst: idle, on: "stop"},
             {src: running, dst: running, on: "tick"}]
   each t in tr where t.src != t.dst { t.src -> t.dst [label: "{t.on}", routing: curved] }
   ```
   Self-transitions use `where t.src == t.dst` with a waypoint pair, as in the sequence example.
3. **Class diagram:**
   - Attributes as records `{id: status, t: "status: EntryStatus", st: changed}` and one
     `each a in attrs { rect a_row [label: a.t, style: a.st] }`. This keeps **source order**.
     The current template needs four parallel lists (`attrs`/`changed`/`added`/`later`) because
     "each iterates plain strings", so a changed attribute can never appear between two
     unchanged ones.
   - Keyed rows give `entry.r_a[billed_at]` instead of `entry.r_a_2`.
   - Associations: `let rels = [{a: corr.t_box, b: entry.b_box, look: inherits}, ...]` then
     `each r in rels { r.a -> r.b [style: r.look] }`. `inherits` is a user-defined style name,
     not an engine notion.
4. **Gantt/timeline dependencies (P1, P2):** `each d in deps { d.before.right -> d.after.left
   [routing: orthogonal] }`.
5. **Cross-functional flow:** steps `{id: s3, lane: ops, text: "Approve"}`, with
   `constrain s.center_y = s.lane.center_y` placing each step in its lane (P2 in a constraint).

Rejected for failing the rule: a `message` statement, a `rows:`/`sequence` container,
auto-assigning a y-position from statement order, and any lint keyed on `kind:`.

## 4. The sequence example after

The `life` template is unchanged. It already does `each t in ticks { rect p ... }`. With keyed
records its rows become `p_login`, ...

```
let msgs = [
  {id: login,    from: cl,   to: api,  kind: call,  label: "login(user)"},
  {id: verify,   from: api,  to: auth, kind: call,  label: "verify"},
  {id: ok,       from: auth, to: api,  kind: reply, label: "ok"},
  {id: validate, from: api,  to: api,  kind: call,  label: "validate cart"},
  {id: query,    from: api,  to: db,   kind: call,  label: "query"},
  {id: rows,     from: db,   to: api,  kind: reply, label: "rows"},
  {id: ok200,    from: api,  to: cl,   kind: reply, label: "200 OK"},
  {id: conflict, from: api,  to: cl,   kind: reply, label: "409"},
  {id: audit,    from: api,  to: db,   kind: async, label: "audit(event)"},
]
// looks, via named modifier sets (concurrent work (a)); syntax is theirs
style call  [routing: direct, label_offset: 0, label_mask: 4]
style reply [routing: direct, label_offset: 0, label_mask: 4, to_end: open, stroke_dasharray: dashed]
style async [routing: direct, label_offset: 0, label_mask: 4, to_end: open]

row actors [gap: 100, align: top] {
  life cl   [ttl: "Client",   ticks: msgs]
  life api  [ttl: "API",      ticks: msgs]
  life auth [ttl: "Auth",     ticks: msgs]     // was 4 rows; see open question 5
  life db   [ttl: "Database", ticks: msgs]
}

// one row per message; a message is an arrow across its own row
each m, i in msgs where m.from != m.to {
  m.from.p[m.id] -> m.to.p[m.id] [style: m.kind, label: "{i+1}: {m.label}"]
}
// a self-message leaves at the top of its row and comes back at the bottom
each m, i in msgs where m.from == m.to {
  point out
  point back
  constrain out.center_x = m.from.p[m.id].center_x + 40
  constrain out.center_y = m.from.p[m.id].top
  constrain back.center_x = out.center_x
  constrain back.center_y = m.from.p[m.id].bottom
  m.from.p[m.id].top -> m.from.p[m.id].bottom [style: m.kind, via: [out, back], label: "{i+1}: {m.label}"]
}

// activation bars from data: which lifeline, first and last row; ids name them act_api, ...
// (alt_f's `overlaps: [..., act_api]` keeps working)
let bars = [{id: api, on: api, first: login, last: conflict}, {id: auth, on: auth, first: verify, last: ok},
            {id: db, on: db, first: query, last: rows}]
each b in bars {
  rect act [corner_radius: 0, width: 10, fill: role-surface-2, stroke: role-muted, overlaps: [b.on, b.on.lifeline]]
  constrain act.center_x = b.on.p[b.first].center_x
  constrain act.top = b.on.p[b.first].center_y - 4
  constrain act.bottom = b.on.p[b.last].center_y + 4
}

// fragments and notes name rows by message, not by number
constrain loop_f contains api.p[query], db.p[rows] [padding_x: 110, padding_y: 0]
constrain alt_f contains cl.p[ok200], api.p[conflict] [padding_x: 80, padding_y: 0]
constrain alt_div.center_y = cl.p[ok200].bottom + 4
constrain note1.center_x = db.p[validate].center_x
constrain note1.center_y = db.p[validate].center_y
// (frame/label/divider shapes and the destroy mark as today, minus the row numbers)
```

**Inserting a message** is now one line in `msgs`. Labels renumber themselves. Frames, bars and
notes reference ids, so they stay put. The self-message now takes one row instead of two, so
row k is message k and the label number is the row number. In today's file these differ: the
self-message spans rows 4–5 and query is "5:" on row 6.

**Not solved:** auth's shorter lifeline. Today it is a 4-item `ticks`. With a shared list, every
lifeline gets all rows (open question 5).

## 5. Alternatives considered and rejected

- **The agent generates the lines itself** (the status quo, which skill.md even suggests). This
  is the failure being fixed: every insertion is a renumbering exercise that agents get wrong.
- **Parallel lists** (`froms: [...], tos: [...]`). skill.md already says "Never parallel lists".
  They drift out of step silently.
- **Star declarations for connections** (`cl.p* -> api.p*`, zipped like
  `constrain k*.center = d*.center`). Zipping connects p_k to p_k on two fixed lifelines.
  Endpoints cannot vary per item, and stars have no body for the self-message.
- **Identifier interpolation at the surface** (`p_{i+1}`). It conflicts with `{` blocks in the
  lexer and is inconsistent with the existing `row[2]` sugar. It survives only as the internal
  form of P3.
- **A message template using element-reference parameters (b)** (`msg m5 [a: api, b: db,
  row: 6]`). It is still one hand-written line per message with a hand row number. It is useful
  *inside* an `each` body, not instead of one.
- **General conditionals (`if`) or expressions in the body.** They are heavier and invite
  logic-in-diagrams. `where` with equality covers the observed needs. Revisit only with
  evidence.
- **A self-loop route instead of P6.** That would be `a -> a [routing: loop]`, and today
  `a -> a` draws a stub. It is a good general feature (state machines need it), but it fixes only
  this one case of per-item variation. `where` covers that case and more. Both may be worth
  having (open question 6).
- **Resolving `p[key]` by searching all lets for the id** (no keyed names). It is cheaper, but
  it silently picks the wrong row when a template is passed a different list than the one
  searched. Keyed names are correct by construction.

## 6. Open questions for the maintainer

1. **Keywords as field names.** Allow `{from: ...}` / `m.from` / `m.label` (recommended,
   agent-natural), or document `src`/`dst` and leave the grammar alone?
2. **`_k` vs `row2`.** Each-copies are `p_3`, table and code parts `row3`/`line3`, and both are
   written `[3]`. Accept the split (recommended: the `[k]` form hides it), or unify the
   built-in parts on `_k` (a breaking change)?
3. **Keyed naming trigger.** Implicit `id:` field (recommended) or explicit `each ... by id`?
4. **Missing fields.** An error (proposed), or allow defaults (`each m in msgs [defaults:
   {kind: call}]`)? Defaults would shorten `msgs` when most items are one kind.
5. **Slices** for a shorter list from a shared one: `ticks: msgs[..validate]`. This reuses the
   `rows[2..4]` range syntax, with key or number bounds, and would restore auth's short
   lifeline. Is it worth a seventh primitive?
6. **Self-loop routing** (`a -> a`) as a separate general connection feature?
7. **`let` scope:** top level only (proposed), or also inside template bodies? May a let hold a
   record or scalar for reuse as a modifier value (that drifts toward variables; proposed: no)?

## 7. Implementation plan (each step ships alone, with docs + example per CLAUDE.md)

Every step updates `docs/skill.md` (the `each` section and the "Reach For These First" table),
`docs/grammar.md`, `docs/examples.md` and the `--doc each` alias, and adds or updates an
example under `examples/`. Run `bash examples/render-all.sh`, and check that all of
`examples/` still renders unchanged after S1 and S3.
- `tests/docs_class_template_current.rs` (commit c502bea) keeps the **class** template quoted in
  `--skill`/`--examples` verbatim, so the class rewrite in S2/S5 must update both docs.
- The sequence example appears in `--examples` as EXAMPLE 12 and must be updated by hand.

| Step | Ships | Tests (tests/each_block.rs unless noted) | Example |
|---|---|---|---|
| **S1** P1: names in every position, `contains`/`midpoint` keep dots until after each, shadowing error, item-span on undefined names | fan-out, any `each` that wires things | `each n in [a,b] { n -> hub }` makes 2 connections; `constrain n.width`; `contains n`; `via: [n]`; dotted value `c: board.led` → `board.led.top`; inside a template (prefixing still right); shadowing error text; typo'd item caret on the list item | `examples/fan-out.ail` (the CLAUDE.md success prompt) |
| **S2** P2: records, `m.f` in values/names/strings, keyword field names | transition tables, class attrs in order (with (a)) | field in label/string/endpoint/constraint; unknown field lists fields + did-you-mean; missing-in-some-items lists item numbers; whole record misuse; `{i+1}: {m.f}` | `examples/state-machine.ail` from a transition list; once (a) lands, the class template moves to one `attrs` record list |
| **S3** P3: `x.p[k]` generalised in `indexed_parts`, `{...}` in identifiers, unbound-index error, `p_0` hint | data-driven messages with numeric rows | `api.p[6]` == `api.p_6` in endpoint, constraint, contains, via list; `p[i+1]` in each; `p[i]` gives the `p_0` hint; `p[i+1]` outside each is an error; `rect a [x]`/`a -> b [auto]` unaffected (adjacency) | a pipeline whose boxes come from one `each` and whose arrows `box[i+1].right -> box[i+2].left` come from another. Optionally the sequence messages via an explicit `row:` field (`p[m.row]`), which still carries hand-written row numbers because the self-message (2 rows) stays outside the list |
| **S4** P4: `let` | shared lists across instances and blocks | let as each source, as instance arg, as nested default; redefinition error; used as style value error; `let` in `--doc reserved` | state machine: `let tr = [...]` shared by the transitions `each` and a legend |
| **S5** P5: keyed copies | insertion-stable references | keyed names `p_login`; `p[login]`; duplicate/mixed/non-word id errors; numeric index on keyed family hint; `i` still positional | class rows by id (`entry.r_a[billed_at]`) |
| **S6** P6: `where` | per-kind bodies | `==`/`!=`/field-vs-field/`and`; `i` and names keep source positions; unknown field in `where` | state-machine self-transitions; the sequence example rewritten as in §4 (it needs S3–S6 together) |

S1 alone already fixes a real gap that affects every diagram type. S2 opens up tables of
records. The sequence example can only be fully data-driven once S3–S6 are all in: one list,
one row per item, the self-message in that list. Until then it keeps its hand-written messages,
or uses the `row:` variant from S3.
