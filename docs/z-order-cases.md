# Z-order pressure cases

Paint order is declaration order (later draws on top), plus automatic rules
(through-lines under their stations). There is no explicit z-order, by choice:
when one is needed, it should be designed from the real cases logged here.

Log a case when a scene needs a particular paint order. Note how it was solved.

## Solved by declaration order (plus a comment)

1. `flow-design.ail`: the iceberg is declared before design.md, so the
   document covers its tip.
2. `flow-issues.ail`: the lanes (opaque bands) come before the issue cards
   that move onto them.
3. `agentic-loop.ail`, `chatbot-llm.ail`: the conversation box comes before
   its messages.
4. `wf-roles.ail`: the accent outline on c4 overlapped c3/c5. This was fixed
   with spacing. Putting the outline under the neighbours was the z-order
   alternative; it was rejected because it would look clipped.

## Would break the implicit model (none hit yet)

- **Order changes during the animation**: a card slides behind something in
  one step and in front of it in the next.
- **Declaration order fights reading order**: an element declared early for
  paint order needs constraints against things declared later, so the file
  reads backwards.

When a scene hits one of these, log it here. Three or more real entries of
the second kind are the evidence to design an explicit mechanism from.
