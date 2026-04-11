# Nom Language Syntax Reference

Formal syntax documentation for the Nom programming language, derived from the
implemented lexer (`nom-lexer`) and parser (`nom-parser`).

Everything documented here compiles. Planned features are not included.

---

## Overview

Nom is a writing-style language. Programs read like English sentences.

- **Classifiers** start every declaration (like Vietnamese noun classifiers).
- **Blank lines** separate declarations. No braces around declaration bodies.
- **No semicolons**, no curly braces for scoping (except branch blocks and implement blocks).
- **Newlines** separate statements within a declaration.
- **Indentation** is optional and purely cosmetic.
- **Comments** start with `#` and run to end of line.

```nom
# This is a comment

system auth
need hash::argon2 where security>0.9
need store::redis where reliability>0.8
flow request->hash->store->response
require latency<50ms
effects only [network database cpu]
```

---

## File Structure

A `.nom` source file is a sequence of declarations separated by blank lines.

```
source_file ::= declaration* EOF
```

Each declaration starts with a classifier keyword followed by a name, then
zero or more statements on subsequent lines. The declaration ends at the next
blank line, the next classifier keyword (starting a new declaration), or EOF.

```nom
system auth
need hash::argon2
require latency<50ms

flow hello
flow "hello world"->print
```

Two declarations: `system auth` and `flow hello`.

---

## Classifiers (10)

Every declaration begins with exactly one classifier keyword. The classifier
determines what kind of entity is being declared.

| Classifier | Purpose | Example |
|------------|---------|---------|
| `system` | Composite service or application | `system auth` |
| `flow` | Data flow or processing pipeline | `flow register` |
| `store` | Persistent storage or state | `store sessions` |
| `graph` | Graph structure with nodes, edges, queries | `graph social` |
| `agent` | Autonomous actor with capabilities | `agent monitor` |
| `test` | Test specification | `test authsecurity` |
| `nom` | Custom word definition (escape hatch) | `nom scorer` |
| `gate` | Access control or routing gate | `gate ratelimit` |
| `pool` | Resource pool or connection pool | `pool connections` |
| `view` | Projection or read model | `view dashboard` |

`gate`, `pool`, and `view` are recognized by the parser but do not yet have
specialized statement types. They accept the common statements (`need`,
`require`, `effects`, `flow`, `describe`).

---

## Operators

### Composition Operators (3)

| Operator | Name | Meaning | Example |
|----------|------|---------|---------|
| `->` | Arrow (flow) | Sequential data flow between steps | `request->hash->store->response` |
| `::` | Double colon (subordinate) | Variant specialization of a word | `hash::argon2` |
| `+` | Plus (coordinate) | Broadening composition | Reserved for future expression use |

### Comparison Operators (6)

Used in `where` constraints and `require` statements.

| Operator | Meaning | Example |
|----------|---------|---------|
| `>` | Greater than | `security>0.9` |
| `<` | Less than | `latency<50ms` |
| `>=` | Greater than or equal | `score>=0.8` |
| `<=` | Less than or equal | `score<=1.0` |
| `=` | Equal | `port=8080` |
| `!=` | Not equal | `status!=0` |

### Delimiters

| Symbol | Usage |
|--------|-------|
| `{` `}` | Branch blocks, implement blocks |
| `[` `]` | Effect lists, capability lists |
| `(` `)` | Parameter lists, function calls |
| `,` | Separator in parameter lists |

---

## System Declaration

Declares a composite service. The primary building block for applications.

**Statements available:** `need`, `require`, `effects`, `flow`, `describe`

```nom
system auth
need hash::argon2 where security>0.9
need store::redis where reliability>0.8
need limiter::tokenbucket where performance>0.9
flow request->limiter->hash->store->response
require latency<50ms
effects only [network database cpu]
```

### need

Import a dictionary word, optionally with a variant and constraint.

```
need_stmt ::= "need" nom_ref ("where" constraint)?
nom_ref   ::= IDENT ("::" IDENT)?
```

```nom
need hash                          # word only
need hash::argon2                  # word with variant
need hash::argon2 where security>0.9  # with constraint
```

### require

Declare a non-functional constraint on the declaration.

```
require_stmt ::= "require" constraint
constraint   ::= expr compare_op expr
```

```nom
require latency<50ms
require reliability>0.99
```

### effects

Declare the side effects this declaration is allowed to produce.

```
effects_stmt ::= "effects" modifier? "[" IDENT* "]"
modifier     ::= "only" | "good" | "bad"
```

```nom
effects only [network database cpu]   # restrict to exactly these effects
effects [network database]            # declare effects (no restriction)
effects good [cachehit]               # positive effects
effects bad [timeout]                 # negative effects
```

### flow (statement)

Define the data flow pipeline within a declaration.

```
flow_stmt  ::= "flow" flow_chain
flow_chain ::= flow_step ("->" flow_step)*
flow_step  ::= nom_ref | literal | branch_block | call_expr
```

```nom
flow request->limiter->hash->store->response
flow "hello world"->print
flow request->validate->{ iftrue->process, iffalse->reject }->response
```

### describe

Attach a human-readable description to a declaration.

```
describe_stmt ::= "describe" STRING
```

```nom
describe "calculate relevance score for legal documents"
```

---

## Flow Declaration

Declares a named data flow pipeline. The `flow` keyword serves double duty:
as a classifier (starting a declaration) and as a statement keyword (inside
any declaration).

When `flow` is followed by an identifier and then `->`, it is parsed as a
flow statement. When followed by an identifier and then a newline or blank
line, it starts a new declaration.

```nom
flow register
need hash::argon2
flow input->validate->hash->store->confirm

flow hello
flow "hello world"->print
```

### Branching

Branch blocks use `{ iftrue->..., iffalse->... }` syntax inside flow chains.

```
branch_block ::= "{" branch_arm ("," branch_arm)* "}"
branch_arm   ::= ("iftrue" | "iffalse" | IDENT) "->" flow_chain
```

```nom
flow request->validate->{ iftrue->process->store, iffalse->reject }->response
```

### Function Calls in Flows

A flow step can be a function call with parenthesized arguments.

```nom
flow user->query(friends)->sort->display
```

---

## Store Declaration

Declares a persistent storage entity.

```nom
store sessions
need store::redis where reliability>0.8
effects only [database]
```

---

## Graph Declaration

Declares a graph structure with typed nodes, edges, queries, and constraints.

**Statements available:** `node`, `edge`, `query`, `constraint`, plus common statements

### node

Define a typed node with named fields.

```
graph_node ::= "node" IDENT "(" typed_param ("," typed_param)* ")"
typed_param ::= IDENT IDENT?
```

```nom
graph social
node user(name text, age number)
node post(title text, body text)
```

### edge

Define a typed edge between nodes.

```
graph_edge ::= "edge" IDENT "(" typed_param ("," typed_param)* ")"
```

The parameters named `from` and `to` are extracted as endpoint types.
All other parameters become edge fields.

```nom
edge follows(from user, to user, weight real)
edge authored(from user, to post)
```

### query

Define a named graph traversal query.

```
graph_query ::= "query" IDENT "(" typed_param* ")" "=" flow_chain
```

```nom
query friends_of(user) = user->follows->user
query posts_by(user) = user->authored->post
```

### constraint

Define an integrity constraint on the graph.

```
graph_constraint ::= "constraint" IDENT "=" expr compare_op expr
```

```nom
constraint no_self_follow = follows.from != follows.to
```

### Complete Graph Example

```nom
graph social
node user(name text, age number)
node post(title text, body text)
edge follows(from user, to user, weight real)
edge authored(from user, to post)
query friends_of(user) = user->follows->user
constraint no_self_follow = follows.from != follows.to
```

---

## Agent Declaration

Declares an autonomous agent with capabilities, supervision, message handling,
state, and scheduling.

**Statements available:** `capability`, `supervise`, `receive`, `state`, `schedule`, plus common statements

### capability

Declare the capabilities (permissions) the agent requires.

```
agent_capability ::= "capability" "[" IDENT* "]"
```

```nom
capability [network observe filesystem]
```

### supervise

Declare the supervision strategy and parameters.

```
agent_supervise ::= "supervise" IDENT (IDENT "=" expr)*
```

```nom
supervise restart_on_failure max_retries=3
```

### receive

Declare how the agent handles incoming messages (as a flow chain).

```
agent_receive ::= "receive" flow_chain
```

```nom
receive message->classify->route
```

### state

Declare the agent's initial state.

```
agent_state ::= "state" IDENT
```

```nom
state active
```

### schedule

Declare a recurring scheduled action.

```
agent_schedule ::= "schedule" "every" INTERVAL flow_chain
```

The interval is a number followed by a unit suffix (e.g., `5m`, `1h`, `30s`).
The lexer produces this as a single identifier token.

```nom
schedule every 5m check_health
```

### Complete Agent Example

```nom
agent monitor
capability [network observe]
supervise restart_on_failure max_retries=3
receive message->classify->route
state active
schedule every 5m check_health
```

---

## Test Declaration

Declares a test specification using given/when/then/and.

**Statements available:** `given`, `when`, `then`, `and`

### given

Set up the test subject with optional configuration.

```
given_stmt ::= "given" IDENT (IDENT "=" expr)*
```

```nom
given auth with store::memory
```

### when

Specify the action under test with optional parameters.

```
when_stmt ::= "when" IDENT (IDENT "=" expr)*
```

```nom
when request with token expired
```

### then

Assert the expected result.

```
then_stmt ::= "then" expr
```

```nom
then response.status = 401
```

### and

Add additional assertions.

```
and_stmt ::= "and" expr
```

```nom
and effects include bad(authfailure)
and not effects in [filesystem]
```

### Complete Test Example

```nom
test authsecurity
given auth with store::memory
when request with token expired
then response
and effects
```

---

## Nom Declaration (Escape Hatch)

The `nom` classifier defines a custom dictionary word with description,
contract, and inline implementation. This is how you extend the dictionary
with new words when no existing .nomtu covers your need.

**Statements available:** `describe`, `contract`, `implement`

### contract

Define the contract (inputs, outputs, effects, pre/postconditions).

```
contract_stmt ::= "contract" contract_body
contract_body ::= (("in" | "input") IDENT IDENT?
                  | ("out" | "output") IDENT IDENT?
                  | "effects" "[" IDENT* "]"
                  | "pre" expr
                  | "post" expr)*
```

### implement

Provide inline implementation in a host language.

```
implement_stmt ::= "implement" IDENT "{" raw_code "}"
```

Everything between `{` and the matching `}` is captured as raw code.
Nested braces are tracked for correct matching.

### Complete Nom Example

```nom
nom scorer
describe "calculate relevance score for legal documents using keyword matching"

contract
in document(text) query(text)
out score(real range 0.0 to 1.0)
effects [cpu]
pre document.length > 0
post deterministic

implement rust {
    fn score(doc: &str, query: &str) -> f32 {
        let tokens: Vec<&str> = doc.split_whitespace().collect();
        if tokens.is_empty() {
            return 0.0;
        }
        let matches = tokens.iter().filter(|t| query.contains(*t)).count();
        matches as f32 / tokens.len() as f32
    }
}
```

---

## Gate, Pool, View Declarations

These classifiers are recognized by the parser and create valid declarations,
but do not yet have specialized statement types. They accept the common
statements: `need`, `require`, `effects`, `flow`, `describe`.

```nom
gate ratelimit
need limiter::tokenbucket
flow request->check->{ iftrue->allow, iffalse->reject }

pool connections
need database::postgres
require max_connections<100

view dashboard
need store::metrics
flow metrics->aggregate->render
```

---

## Constraints

Constraints appear in `where` clauses (after `need`) and `require` statements.
A constraint is a comparison between two expressions.

```
constraint  ::= expr compare_op expr
compare_op  ::= ">" | "<" | ">=" | "<=" | "=" | "!="
```

The left-hand side is typically an identifier (a quality score name or metric).
The right-hand side is typically a literal (number or identifier with unit).

```nom
need hash::argon2 where security>0.9
require latency<50ms
require reliability>0.99
need http::server where port=8080
```

---

## Effects

Effects declare the side effects a declaration is permitted or expected to produce.

```
effects_stmt ::= "effects" modifier? "[" IDENT* "]"
modifier     ::= "only" | "good" | "bad"
```

| Modifier | Meaning |
|----------|---------|
| `only` | Restrict to exactly these effects (whitelist) |
| `good` | Positive/expected effects |
| `bad` | Negative/failure effects |
| (none) | Declare effects without restriction |

Common effect names: `network`, `database`, `filesystem`, `cpu`, `clock`, `random`.

```nom
effects only [network database cpu]
effects good [cachehit]
effects bad [timeout]
```

---

## Literals

### Integers

Whole numbers. Parsed as 64-bit signed integers.

```nom
require max_connections<100
```

### Floats

Decimal numbers. Parsed as 64-bit floating point.

```nom
need hash::argon2 where security>0.9
```

### Strings

Double-quoted string literals with escape sequences (`\n`, `\t`, `\"`, `\\`).

```nom
describe "calculate relevance score for legal documents"
flow "hello world"->print
```

### Number+Unit Identifiers

Numbers followed immediately by alphabetic characters are lexed as a single
identifier token. This is how unit-suffixed values like `50ms`, `5m`, `1gb`
are represented.

```nom
require latency<50ms
schedule every 5m check_health
```

---

## Comments

Line comments start with `#` and extend to end of line.

```nom
# This is a comment
system auth        # inline comment
need hash::argon2  # import the argon2 hashing word
```

Comments are preserved in the token stream but skipped during parsing.

---

## Grammar (BNF-style)

Complete formal grammar matching the parser implementation.

```
source_file     ::= declaration* EOF

declaration     ::= classifier IDENT statement* (BLANK_LINE | classifier | EOF)

classifier      ::= "system" | "flow" | "store" | "graph" | "agent"
                   | "test" | "nom" | "gate" | "pool" | "view"

statement       ::= need_stmt | require_stmt | effects_stmt | flow_stmt
                   | describe_stmt | contract_stmt | implement_stmt
                   | given_stmt | when_stmt | then_stmt | and_stmt
                   | graph_node | graph_edge | graph_query | graph_constraint
                   | agent_capability | agent_supervise | agent_receive
                   | agent_state | agent_schedule

need_stmt       ::= "need" nom_ref ("where" constraint)?
require_stmt    ::= "require" constraint
effects_stmt    ::= "effects" ("only" | "good" | "bad")? "[" IDENT* "]"
flow_stmt       ::= "flow" flow_chain
describe_stmt   ::= "describe" STRING

contract_stmt   ::= "contract" contract_line*
contract_line   ::= ("in" | "input") IDENT IDENT?
                   | ("out" | "output") IDENT IDENT?
                   | "effects" "[" IDENT* "]"
                   | "pre" expr
                   | "post" expr

implement_stmt  ::= "implement" IDENT "{" raw_code "}"

given_stmt      ::= "given" IDENT (IDENT "=" expr)*
when_stmt       ::= "when" IDENT (IDENT "=" expr)*
then_stmt       ::= "then" expr
and_stmt        ::= "and" expr

graph_node      ::= "node" IDENT typed_param_list?
graph_edge      ::= "edge" IDENT typed_param_list?
graph_query     ::= "query" IDENT typed_param_list? "=" flow_chain
graph_constraint::= "constraint" IDENT "=" constraint_expr

agent_capability::= "capability" "[" IDENT* "]"
agent_supervise ::= "supervise" IDENT (IDENT "=" expr)*
agent_receive   ::= "receive" flow_chain
agent_state     ::= "state" IDENT
agent_schedule  ::= "schedule" "every" IDENT flow_chain

nom_ref         ::= IDENT ("::" IDENT)?
constraint      ::= expr compare_op expr
constraint_expr ::= expr (compare_op expr)?
compare_op      ::= ">" | "<" | ">=" | "<=" | "=" | "!="

flow_chain      ::= flow_step ("->" flow_step)*
flow_step       ::= nom_ref
                   | STRING | INTEGER | FLOAT
                   | "{" branch_arm ("," branch_arm)* "}"
                   | IDENT "(" (expr ("," expr)*)? ")"
branch_arm      ::= ("iftrue" | "iffalse" | IDENT) "->" flow_chain

typed_param_list::= "(" typed_param ("," typed_param)* ")"
typed_param     ::= IDENT IDENT?

expr            ::= IDENT | INTEGER | FLOAT | STRING

IDENT           ::= [a-zA-Z_][a-zA-Z0-9_]*
INTEGER         ::= [0-9]+
FLOAT           ::= [0-9]+ "." [0-9]*
STRING          ::= '"' (escape | [^"\\])* '"'
escape          ::= '\\' [ntr"\\]
BLANK_LINE      ::= NEWLINE NEWLINE+
COMMENT         ::= '#' [^\n]*
```

---

## Complete Examples

### auth.nom -- Authentication Service

Compiles to an 834KB native Windows binary with `nom build auth.nom`.

```nom
# Authentication service: hash passwords, store sessions, rate limit

system auth
need hash::argon2 where security>0.9
need store::redis where reliability>0.8
need limiter::tokenbucket where performance>0.9
flow request->limiter->hash->store->response
require latency<50ms
effects only [network database cpu]
```

### hello.nom -- Minimal Program

```nom
# The simplest Nom program: print hello world

flow hello
flow "hello world"->print
```

### custom_word.nom -- Custom Dictionary Word

```nom
# Define a custom .nomtu word with inline Rust implementation

nom scorer
describe "calculate relevance score for legal documents using keyword matching"

contract
in document(text) query(text)
out score(real range 0.0 to 1.0)
effects [cpu]
pre document.length > 0
post deterministic

implement rust {
    fn score(doc: &str, query: &str) -> f32 {
        let tokens: Vec<&str> = doc.split_whitespace().collect();
        if tokens.is_empty() {
            return 0.0;
        }
        let matches = tokens.iter().filter(|t| query.contains(*t)).count();
        matches as f32 / tokens.len() as f32
    }
}
```

### test_auth.nom -- Test Specification

```nom
# Test the auth service with expired token

test authsecurity
given auth with store::memory
when request with token expired
then response.status = 401
and effects include bad(authfailure)
and not effects in [filesystem]
```

### webapi.nom -- Web API with Branching

```nom
# Simple web API: serve user profiles from a database

system userapi
need http::server where port=8080
need database::postgres
need auth::jwt
flow /users/{id}->auth->database->response
iftrue response 200
iffalse response 404
```

### Graph Example

```nom
graph social
node user(name text, age number)
node post(title text, body text)
edge follows(from user, to user, weight real)
edge authored(from user, to post)
query friends_of(user) = user->follows->user
constraint no_self_follow = follows.from != follows.to
```

### Agent Example

```nom
agent monitor
capability [network observe]
supervise restart_on_failure max_retries=3
receive message->classify->route
state active
schedule every 5m check_health
```

---

## Keyword Reference

All 42 keywords recognized by the lexer, grouped by category.

### Classifiers (10)
`system` `flow` `store` `graph` `agent` `test` `nom` `gate` `pool` `view`

### Declaration (6)
`need` `require` `effects` `where` `only` `describe`

### Flow (3)
`branch` `iftrue` `iffalse`

### Test (6)
`given` `when` `then` `and` `contract` `implement`

### Graph (4)
`node` `edge` `query` `constraint`

### Agent (6)
`capability` `supervise` `receive` `state` `schedule` `every`
