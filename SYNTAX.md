# Nom Language Syntax Reference

Formal syntax documentation for the Nom programming language, derived from the
implemented lexer (`nom-lexer`) and parser (`nom-parser`).
Everything documented here compiles. Planned features are not included.


---

## Overview

Nom is a hybrid declarative-imperative language. Programs combine orchestration
(flows, constraints, effects) with general-purpose code (functions, types, control flow).

- **Classifiers** start every declaration (like Vietnamese noun classifiers).
- **Blank lines** separate declarations. No braces around declaration bodies.
- **Declarative statements** (`need`, `require`, `effects`, `flow`) describe composition.
- **Imperative statements** (`let`, `fn`, `if`, `for`, `struct`, `enum`) describe logic.
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

Define the data flow pipeline within a declaration. Flows support qualifiers (once, stream, scheduled)
and fault handling strategies (onfail).

```
flow_stmt   ::= "flow" flow_qualifier? flow_chain flow_onfail?
flow_qualifier ::= "::" ("once" | "stream" | "scheduled")
flow_chain  ::= flow_step ("->" flow_step)*
flow_step   ::= nom_ref | literal | branch_block | call_expr
flow_onfail ::= "onfail" onfail_strategy
onfail_strategy ::= "abort"
                  | "retry" INTEGER
                  | "restart_from" nom_ref
                  | "skip"
                  | "escalate"
```

```nom
flow request->limiter->hash->store->response
flow "hello world"->print
flow request->validate->{ iftrue->process, iffalse->reject }->response

# With qualifiers
flow::once request->hash->store         # Run once (idempotent)
flow::stream events->process->output    # Ongoing, produces values
flow::scheduled daily->cleanup->report  # Runs on schedule

# With fault handling
flow request->hash->store onfail abort              # Stop on failure (default)
flow request->hash->store onfail retry 3           # Retry up to 3 times
flow request->hash->store onfail restart_from hash # Restart from hash node
flow request->hash->store onfail skip              # Skip failed node
flow request->hash->store onfail escalate          # Escalate to parent
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

### Flow Qualifiers (ADOPT-5)

Qualifiers control the execution mode of a flow.

| Qualifier | Meaning | Use case |
|-----------|---------|----------|
| `::once` | Idempotent, runs exactly once | Authentication, registration |
| `::stream` | Produces values over time | Event processing, data pipeline |
| `::scheduled` | Runs on a schedule | Daily cleanup, periodic reports |

```nom
flow::once request->hash->store
flow::stream events->classify->aggregate
flow::scheduled daily->cleanup->report
```

### Flow Fault Handling (ADOPT-4)

Fault handling strategies define what happens when a flow step fails.

| Strategy | Behavior |
|----------|----------|
| `onfail abort` | Stop immediately (default) |
| `onfail retry N` | Retry up to N times |
| `onfail restart_from node` | Restart from a specific node |
| `onfail skip` | Skip the failed node and continue |
| `onfail escalate` | Escalate to parent flow |

```nom
flow request->hash->store onfail abort
flow request->hash->store onfail retry 3
flow request->hash->store onfail restart_from hash
flow request->hash->store onfail skip
flow request->hash->store onfail escalate
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
graph_query      ::= "query" IDENT "(" typed_param* ")" "=" graph_query_expr
graph_query_expr ::= graph_query_primary ("->" IDENT "->" graph_query_primary)*
graph_query_primary ::= IDENT
                      | "{" branch_arm+ "}"
                      | ("union" | "intersect" | "difference") "(" graph_query_expr ("," graph_query_expr)+ ")"
                      | "(" graph_query_expr ")"
```

```nom
query friends_of(user) = user->follows->user
query posts_by(user) = user->authored->post
query common_friends(a user, b user) = intersect(a->follows->user, b->follows->user)
query visible_posts(a user, b user) = difference(union(a->follows->user, b->follows->user), a)->authored->post
```

Legacy branch blocks remain accepted for compatibility and are lowered to
union/intersection set algebra internally.

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
query common_friends(a user, b user) = intersect(a->follows->user, b->follows->user)
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
contract_body ::= (("in" | "input") contract_param+
                  | ("out" | "output") contract_param+
                  | "effects" "[" IDENT* "]"
                  | "pre" expr
                  | "post" expr)*
contract_param ::= IDENT IDENT?
                 | IDENT "(" IDENT annotation* ")"
```

Both `in document text` and `in document(text)` forms are accepted. Parenthesized
contract params may include extra annotations after the first type token; the
compiler keeps the primary type and ignores the trailing annotation payload for now.

### implement

Provide inline implementation in a host language.

```
implement_stmt ::= "implement" IDENT "{" raw_code "}"
```

Everything between `{` and the matching `}` is captured byte-for-byte as raw code.
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
Within expressions, Nom now supports field access (`document.length`),
calls (`score(query)`), arithmetic precedence (`a + b * 2`), grouping with
parentheses, and boolean composition with `and` / `or`.

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
contract_line   ::= ("in" | "input") contract_param+
                   | ("out" | "output") contract_param+
                   | "effects" "[" IDENT* "]"
                   | "pre" expr
                   | "post" expr
contract_param  ::= IDENT IDENT?
                   | IDENT "(" IDENT annotation* ")"

implement_stmt  ::= "implement" IDENT "{" raw_code "}"

given_stmt      ::= "given" IDENT (IDENT "=" expr)*
when_stmt       ::= "when" IDENT (IDENT "=" expr)*
then_stmt       ::= "then" expr
and_stmt        ::= "and" expr

graph_node      ::= "node" IDENT typed_param_list?
graph_edge      ::= "edge" IDENT typed_param_list?
graph_query     ::= "query" IDENT typed_param_list? "=" graph_query_expr
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

flow_stmt       ::= "flow" flow_qualifier? flow_chain flow_onfail?
flow_qualifier  ::= "::" ("once" | "stream" | "scheduled")
flow_chain      ::= flow_step ("->" flow_step)*
flow_step       ::= nom_ref
                   | STRING | INTEGER | FLOAT
                   | "{" branch_arm ("," branch_arm)* "}"
                   | IDENT "(" (expr ("," expr)*)? ")"
flow_onfail     ::= "onfail" onfail_strategy
onfail_strategy ::= "abort" | "retry" INTEGER | "restart_from" nom_ref | "skip" | "escalate"

graph_query_expr ::= graph_query_primary ("->" IDENT "->" graph_query_primary)*
graph_query_primary ::= nom_ref
                      | "{" branch_arm ("," branch_arm)* "}"
                      | graph_set_call
                      | "(" graph_query_expr ")"
graph_set_call  ::= ("union" | "intersect" | "difference") "(" graph_query_expr ("," graph_query_expr)+ ")"
branch_arm      ::= ("iftrue" | "iffalse" | IDENT) "->" flow_chain

typed_param_list::= "(" typed_param ("," typed_param)* ")"
typed_param     ::= IDENT IDENT?

expr            ::= logical_or
logical_or      ::= logical_and ("or" logical_and)*
logical_and     ::= comparison ("and" comparison)*
comparison      ::= additive (compare_op additive)*
additive        ::= multiplicative (("+" | "-") multiplicative)*
multiplicative  ::= postfix (("*" | "/") postfix)*
postfix         ::= primary (("." IDENT) | call_suffix)*
call_suffix     ::= "(" (expr ("," expr)*)? ")"
primary         ::= IDENT | INTEGER | FLOAT | STRING | "true" | "false" | "none" | "(" expr ")"

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
query common_friends(a user, b user) = intersect(a->follows->user, b->follows->user)
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

## Imperative Features

Nom supports general-purpose imperative programming alongside its declarative
orchestration syntax. These features can be used inside any declaration body.

### Variables

```
let_stmt ::= "let" "mut"? IDENT (":" type_expr)? "=" expr
```

```nom
nom math
  let x: number = 42
  let mut counter = 0
  let name: text = "hello"
```

### Type Annotations

Types can annotate variables, function parameters, and return values.

```
type_expr ::= named_type | generic_type | function_type | tuple_type | ref_type | "()"
named_type    ::= IDENT                        # text, number, bool, bytes
generic_type  ::= IDENT "[" type_expr ("," type_expr)* "]"   # list[text], map[text, number]
function_type ::= "fn" "(" type_expr* ")" "->" type_expr
tuple_type    ::= "(" type_expr ("," type_expr)+ ")"
ref_type      ::= "&" "mut"? type_expr
```

Built-in type mappings to Rust:

| Nom type | Rust type |
|----------|-----------|
| `text`, `string` | `String` |
| `number`, `real`, `float` | `f64` |
| `integer`, `int` | `i64` |
| `bool`, `boolean` | `bool` |
| `bytes` | `Vec<u8>` |
| `list[T]` | `Vec<T>` |
| `map[K, V]` | `HashMap<K, V>` |
| `option[T]` | `Option<T>` |
| `result[T, E]` | `Result<T, E>` |

### Functions

```
fn_def ::= "pub"? "async"? "fn" IDENT "(" (param ("," param)*)? ")" ("->" type_expr)? block
param  ::= IDENT ":" type_expr
```

```nom
nom math
  fn add(a: number, b: number) -> number {
    let result = a + b
    return result
  }

  pub fn multiply(x: number, y: number) -> number {
    x * y
  }
```

### Control Flow

#### if / else if / else

```
if_expr ::= "if" expr block ("else" "if" expr block)* ("else" block)?
```

```nom
nom logic
  if score > 0.9 {
    let grade = "A"
  } else if score > 0.7 {
    let grade = "B"
  } else {
    let grade = "C"
  }
```

#### for ... in

```
for_stmt ::= "for" IDENT "in" expr block
```

```nom
nom iteration
  for item in items {
    process(item)
  }
```

#### while

```
while_stmt ::= "while" expr block
```

```nom
nom loops
  while count < 10 {
    count = count + 1
  }
```

### Pattern Matching

```
match_expr ::= "match" expr "{" (pattern "=>" block)* "}"
pattern    ::= "_" | literal | IDENT | IDENT "(" pattern ("," pattern)* ")"
```

```nom
nom matching
  match color {
    Red => { let hex = "#FF0000" }
    Green => { let hex = "#00FF00" }
    Blue => { let hex = "#0000FF" }
    _ => { let hex = "#000000" }
  }
```

### Struct Definitions

```
struct_def ::= "pub"? "struct" IDENT "{" (field_def ("," | NEWLINE))* "}"
field_def  ::= "pub"? IDENT ":" type_expr
```

```nom
nom types
  struct Point {
    x: number,
    y: number
  }

  pub struct User {
    pub name: text,
    pub email: text,
    age: integer
  }
```

### Enum Definitions

```
enum_def    ::= "pub"? "enum" IDENT "{" (variant ("," | NEWLINE))* "}"
variant     ::= IDENT ("(" type_expr ("," type_expr)* ")")?
```

```nom
nom types
  enum Color {
    Red,
    Green,
    Blue,
    Custom(integer, integer, integer)
  }

  enum Result {
    Ok(text),
    Err(text)
  }
```

### Blocks

Blocks are delimited by `{` and `}` and contain zero or more statements.
The last expression in a block (without semicolon) is the implicit return value.

```
block      ::= "{" block_stmt* "}"
block_stmt ::= let_stmt | if_expr | for_stmt | while_stmt | match_expr
             | "return" expr? | "break" | "continue" | expr
```

### Hybrid Example

Nom's power comes from combining declarative orchestration with imperative logic:

```nom
system api
need hash::argon2 where security>0.9
need store::redis where reliability>0.8
effects only [network database cpu]
flow request->validate->hash->store->response
require latency<50ms

nom validate
  describe "validate incoming request"
  contract
  in request(bytes)
  out validated(bool)
  effects [cpu]

  fn validate(input: bytes) -> bool {
    if input.len() > 0 {
      let header = parse_header(input)
      return header.is_valid()
    }
    return false
  }
```

---

## Natural Language Aliases

Nom supports natural language keyword aliases inspired by Vietnamese grammar principles.
These aliases map to the same tokens as their technical counterparts, so both styles
can be mixed freely.

| Natural | Technical | Purpose |
|---------|-----------|---------|
| `define` | `fn` | Function definition |
| `kind` | `struct` | Data type with named fields |
| `choice` | `enum` | Sum type with variants |
| `take` / `set` | `let` | Variable binding |
| `changing` | `mut` | Mutable marker |
| `give` / `produce` | `return` | Return a value |
| `each` | `for` | Iteration |
| `repeat` | `while` | Loop with condition |
| `check` | `match` | Pattern matching |
| `share` | `pub` | Public visibility |
| `group` | `mod` | Module grouping |
| `behavior` | `trait` | Interface/behavior definition |
| `apply` | `impl` | Implementation |
| `yes` / `no` | `true` / `false` | Boolean literals |
| `nothing` | `none` | Null/absent value |

### Natural Language Example

```nom
nom geometry
  kind Point {
    x: number,
    y: number
  }
  choice Shape {
    Circle(number),
    Rectangle(number, number)
  }
  define distance(a: Point, b: Point) -> number {
    take dx = a.x - b.x
    take dy = a.y - b.y
    give dx * dx + dy * dy
  }
```

### Design Principles (from Vietnamese Grammar)

1. **Classifier-first** — Every declaration starts with a classifier: `system`, `flow`, `nom`, etc.
2. **Topic-comment** — Name the subject first, then describe it with statements.
3. **Serial verb** — Actions chain: `flow request -> hash -> store -> response`
4. **Modifier-follows-head** — Constraints follow the need: `need hash where security > 0.9`
5. **No inflection** — Keywords never change form. `define` is always `define`.
6. **Foreign borrowing** — Any English word can be a nomtu. The dictionary absorbs all languages.

---

## Keyword Reference

All 100+ keywords recognized by the lexer, grouped by category.

### Classifiers (10)
`system` `flow` `store` `graph` `agent` `test` `nom` `gate` `pool` `view`

### Declaration (6)
`need` `require` `effects` `where` `only` `describe`

### Flow (9)
`branch` `iftrue` `iffalse` `once` `stream` `scheduled` `onfail` `retry` `escalate`

### Test (6)
`given` `when` `then` `and` `contract` `implement`

### Graph (4)
`node` `edge` `query` `constraint`

### Agent (6)
`capability` `supervise` `receive` `state` `schedule` `every`

### Imperative (20)
`let` `mut` `if` `else` `for` `while` `loop` `match` `return` `break`
`continue` `fn` `type` `struct` `enum` `use` `pub` `in` `as` `mod`

### Object-Oriented (4)
`trait` `impl` `self` `async` `await`

### Natural Language Aliases (15)
`define`→fn `kind`→struct `choice`→enum `take`/`set`→let `changing`→mut
`give`/`produce`→return `each`→for `repeat`→while `check`→match
`share`→pub `group`→mod `behavior`→trait `apply`→impl
`yes`→true `no`→false `nothing`→none

### Operators (23)
`->` `=>` `::` `+` `-` `*` `/` `%` `.` `>` `<` `>=` `<=` `=` `==` `!=`
`:` `;` `&` `|` `!` `{` `}` `[` `]` `(` `)` `,`
