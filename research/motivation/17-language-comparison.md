# Part 17: Nom vs. The World — A Language-by-Language Comparison

**Side-by-side comparison of Nom against mainstream programming languages.
Where Nom is genuinely different, where it overlaps, and where incumbents
still win.**

---

## 1. Comparison Framework

Every comparison uses four axes:

```
SYNTAX         How you write code
COMPOSITION    How pieces connect
SAFETY         What the system guarantees
ECOSYSTEM      What you can build today
```

Each language gets an honest assessment. Nom's weaknesses are listed alongside
its strengths.

---

## 2. Nom vs. Rust

### The Relationship

Nom compiles **through** Rust. Rust is the backend, not the competitor.
But developers will ask "why not just write Rust?" — this answers that.

### Syntax

```rust
// Rust: 47 lines for an auth service
use argon2::{self, Config};
use redis::AsyncCommands;
use tower::limit::RateLimitLayer;

struct AuthService {
    hasher: ArgonHasher,
    store: RedisConnection,
    limiter: RateLimiter,
}

impl AuthService {
    async fn authenticate(&self, req: AuthRequest) -> Result<AuthResponse, AuthError> {
        self.limiter.check(&req)?;
        let hash = self.hasher.hash(req.password.as_bytes())?;
        self.store.set(&req.username, &hash).await?;
        Ok(AuthResponse { token: generate_token() })
    }
}
```

```nom
# Nom: 6 lines for the same auth service
system auth
need hash::argon2 where security>0.9
need store::redis where reliability>0.8
need limiter::tokenbucket where performance>0.9
flow request->limiter->hash->store->response
require latency<50ms
```

### Comparison Table

| Dimension | Rust | Nom |
|-----------|------|-----|
| **Verbosity** | Explicit types, lifetimes, trait bounds | 6 lines describe intent, compiler fills the rest |
| **Learning curve** | Steep (ownership, borrowing, lifetimes) | Minimal (write sentences, not programs) |
| **Control** | Total (unsafe, inline asm, custom allocators) | None (engine decides implementation strategy) |
| **Performance** | Manual optimization possible | Engine applies graph-level optimizations automatically |
| **Ecosystem** | 165K+ crates, battle-tested | 0 published .nomtu — dictionary is aspirational |
| **Safety** | Compile-time memory safety via borrow checker | Contract verification + effect tracking |
| **Error messages** | Improving but still complex | Natural-language diagnostics (planned) |
| **Debug** | Full DWARF, GDB/LLDB | Not yet available |

### Verdict

```
USE RUST WHEN:  you need fine-grained control, unsafe code, custom allocators,
                or your domain has no .nomtu coverage yet (which is everything today).

USE NOM WHEN:   your problem is composing well-known components (APIs, services,
                pipelines) and you want verified composition over manual wiring.

THE HONEST GAP: Rust has 165K crates. Nom has 0 published .nomtu.
                Until the dictionary exists, this comparison is theoretical.
```

---

## 3. Nom vs. Python

### Syntax

```python
# Python: auth service with Flask
from flask import Flask, request, jsonify
from argon2 import PasswordHasher
import redis

app = Flask(__name__)
ph = PasswordHasher()
r = redis.Redis(host='localhost', port=6379)

@app.route('/auth', methods=['POST'])
def authenticate():
    data = request.get_json()
    try:
        hash = ph.hash(data['password'])
        r.set(data['username'], hash)
        return jsonify({'status': 'ok'}), 200
    except Exception as e:
        return jsonify({'error': str(e)}), 500
```

```nom
# Nom: same service
system auth
need hash::argon2 where security>0.9
need store::redis where reliability>0.8
flow request->hash->store->response
require latency<50ms
```

### Comparison Table

| Dimension | Python | Nom |
|-----------|--------|-----|
| **Typing** | Dynamic (optional type hints) | Contract-based (in/out/effects verified at compile time) |
| **Performance** | Interpreted, GIL limits concurrency | Native binary via LLVM (no GIL, no interpreter) |
| **Security** | No built-in supply chain verification | Every .nomtu has provenance + scores |
| **AI/ML** | NumPy, TensorFlow, PyTorch — unmatched | No ML ecosystem |
| **Web** | Django, Flask, FastAPI — mature | No web framework |
| **Package management** | pip + PyPI (320K packages) | .nomtu dictionary (0 entries) |
| **Prototyping** | Excellent — REPL, dynamic, forgiving | Requires compilation, less exploratory |
| **Effect tracking** | None — any function can do anything | Effects declared and verified per .nomtu |
| **Runtime errors** | Common (AttributeError, TypeError at runtime) | Caught at compile time via contracts |

### Verdict

```
USE PYTHON WHEN: data science, ML, scripting, prototyping, glue code,
                 or anywhere you need the massive ecosystem.

USE NOM WHEN:    you want verified composition with native performance
                 and don't need Python's specific libraries.

THE HONEST GAP:  Python owns AI/ML. Nom has no answer for NumPy/PyTorch.
                 Python's 320K packages vs. Nom's 0 .nomtu.
```

---

## 4. Nom vs. Go

### Syntax

```go
// Go: auth service
package main

import (
    "net/http"
    "github.com/go-redis/redis/v8"
    "golang.org/x/crypto/argon2"
)

type AuthService struct {
    rdb *redis.Client
}

func (s *AuthService) Handler(w http.ResponseWriter, r *http.Request) {
    password := r.FormValue("password")
    hash := argon2.IDKey([]byte(password), salt, 1, 64*1024, 4, 32)
    s.rdb.Set(ctx, r.FormValue("username"), hash, 0)
    w.WriteHeader(http.StatusOK)
}
```

```nom
system auth
need hash::argon2 where security>0.9
need store::redis where reliability>0.8
flow request->hash->store->response
require latency<50ms
```

### Comparison Table

| Dimension | Go | Nom |
|-----------|-----|-----|
| **Concurrency** | Goroutines + channels (explicit) | Automatic from flow topology (branch/merge) |
| **Compilation** | Fast (seconds) | Depends on Rust backend (slower) |
| **Simplicity** | Intentionally simple, 25 keywords | 10 classifiers, 42 keywords, writing-style |
| **Error handling** | `if err != nil` (verbose, explicit) | Contract pre/post conditions |
| **Generics** | Added in 1.18, still limited | Contracts are polymorphic by design |
| **Ecosystem** | Docker, Kubernetes, cloud-native tools | No infrastructure tooling |
| **Binary size** | Small static binaries | Depends on generated Rust |
| **Dependency management** | Go modules, checksum database | Content-addressed .nomtu with hashes |
| **Supply chain** | sumdb provides integrity | .nomtu adds provenance + scores on top |

### Verdict

```
USE GO WHEN:     cloud infrastructure, DevOps tooling, microservices where
                 simplicity and fast compilation matter.

USE NOM WHEN:    you want the compiler to handle concurrency strategy and
                 dependency verification automatically.

THE HONEST GAP:  Go built Docker and Kubernetes. Nom has built nothing.
                 Go compiles in seconds. Nom's Rust backend is slower.
```

---

## 5. Nom vs. TypeScript

### Syntax

```typescript
// TypeScript: auth service with Express
import express from 'express';
import argon2 from 'argon2';
import Redis from 'ioredis';

const app = express();
const redis = new Redis();

app.post('/auth', async (req, res) => {
  try {
    const hash = await argon2.hash(req.body.password);
    await redis.set(req.body.username, hash);
    res.json({ status: 'ok' });
  } catch (err) {
    res.status(500).json({ error: err.message });
  }
});
```

```nom
system auth
need hash::argon2 where security>0.9
need store::redis where reliability>0.8
flow request->hash->store->response
require latency<50ms
```

### Comparison Table

| Dimension | TypeScript | Nom |
|-----------|------------|-----|
| **Type system** | Structural types, generics, union/intersection | Contracts: in/out/effects/pre/post |
| **Runtime** | Node.js/Deno/Bun (JIT compiled) | Native binary (AOT compiled via LLVM) |
| **npm ecosystem** | 3.2M packages, 2.6B downloads/week | 0 .nomtu |
| **Supply chain** | Major attack vector ($60B losses) | Content-addressed, scored, provenance-tracked |
| **Frontend** | React, Vue, Angular — dominant | No frontend story |
| **Adoption path** | Superset of JS (zero migration cost) | New language (full migration required) |
| **Tooling** | VS Code built-in, excellent LSP | Minimal CLI only |
| **Effect tracking** | None (Promise is the only signal) | Explicit good/bad effects per .nomtu |
| **Bundle size** | Tree-shaking, code splitting | Single native binary |

### Verdict

```
USE TYPESCRIPT WHEN: web frontend, full-stack apps, anywhere JS runs,
                     or when you need npm's 3.2M packages.

USE NOM WHEN:        you want native performance with verified composition
                     and supply chain guarantees.

THE HONEST GAP:      TypeScript owns the web. Nom has no browser story.
                     npm's 3.2M packages vs. Nom's 0 .nomtu.
```

---

## 6. Nom vs. Java / Kotlin

### Syntax

```java
// Java: auth service with Spring Boot
@RestController
public class AuthController {
    @Autowired private PasswordEncoder encoder;
    @Autowired private RedisTemplate<String, String> redis;

    @PostMapping("/auth")
    public ResponseEntity<?> authenticate(@RequestBody AuthRequest req) {
        String hash = encoder.encode(req.getPassword());
        redis.opsForValue().set(req.getUsername(), hash);
        return ResponseEntity.ok(Map.of("status", "ok"));
    }
}
```

```nom
system auth
need hash::argon2 where security>0.9
need store::redis where reliability>0.8
flow request->hash->store->response
require latency<50ms
```

### Comparison Table

| Dimension | Java/Kotlin | Nom |
|-----------|-------------|-----|
| **Paradigm** | Object-oriented (classes, inheritance, interfaces) | Compositional-semantic (classifiers, flows, contracts) |
| **Runtime** | JVM (GC, JIT, warm-up time) | Native binary (no GC, no warm-up) |
| **Enterprise** | Spring, Jakarta EE, massive enterprise adoption | No enterprise story |
| **Boilerplate** | Verbose (annotations, DI, getters/setters) | Minimal (6 lines vs. 20+) |
| **Ecosystem** | Maven Central: 500K+ artifacts | 0 .nomtu |
| **Dependency injection** | Spring DI, Guice, Dagger (runtime/compile-time) | `need` + `where` is declarative DI at language level |
| **Performance** | Good after JIT warm-up, GC pauses | Native, no GC, arena allocation |
| **Backward compat** | Legendary (Java 1.0 code still compiles) | No users, no compat promise yet |

### Verdict

```
USE JAVA/KOTLIN WHEN: enterprise applications, Android, anywhere the JVM
                      ecosystem and long-term stability matter.

USE NOM WHEN:         you want declarative composition without OOP ceremony
                      and native performance without GC pauses.

THE HONEST GAP:       Java has 30 years of enterprise trust. Nom has 0.
                      JVM ecosystem is unmatched in enterprise depth.
```

---

## 7. Nom vs. Haskell / Functional Languages

### Syntax

```haskell
-- Haskell: auth flow (simplified)
authenticate :: Request -> ExceptT AuthError IO Response
authenticate req = do
  limited <- rateLimit req
  hashed  <- hashPassword (password limited)
  stored  <- storeSession (username limited) hashed
  pure (Response stored)
```

```nom
system auth
need hash::argon2 where security>0.9
need store::redis where reliability>0.8
need limiter::tokenbucket where performance>0.9
flow request->limiter->hash->store->response
require latency<50ms
```

### Comparison Table

| Dimension | Haskell | Nom |
|-----------|---------|-----|
| **Type system** | Hindley-Milner, type classes, GADTs, dependent types | Contracts (simpler, domain-focused) |
| **Effects** | Monads (IO, State, Reader, etc.) | Declared effects with good/bad valence |
| **Purity** | Enforced (pure by default, IO monad for effects) | Effects tracked but not enforced via type system |
| **Composition** | Function composition (`.`, `>>=`, `<$>`) | Flow composition (`->`) |
| **Learning curve** | Very steep (monads, functors, applicatives) | Low (write English sentences) |
| **Performance** | Lazy evaluation can cause space leaks | Strict, native, predictable |
| **Ecosystem** | Hackage ~17K packages, niche adoption | 0 .nomtu |
| **Abstraction power** | Extremely high (higher-kinded types, type families) | Intentionally limited (contracts, not type theory) |
| **Readability** | Elegant but dense for non-FP developers | Reads like English |

### Verdict

```
USE HASKELL WHEN:    you want mathematical rigor, advanced type-level
                     programming, or formal verification approaches.

USE NOM WHEN:        you want effect tracking and composition without
                     the steep learning curve of monadic programming.

THE HONEST GAP:      Haskell's type system is vastly more powerful.
                     Nom trades power for accessibility deliberately.
```

---

## 8. Nom vs. SQL / Declarative Languages

### The Parallel

SQL and Nom share a philosophy: **declare what you want, not how to get it.**

```sql
-- SQL: get active users with recent orders
SELECT u.name, COUNT(o.id) as order_count
FROM users u
JOIN orders o ON u.id = o.user_id
WHERE u.active = true
  AND o.created_at > NOW() - INTERVAL '30 days'
GROUP BY u.name
HAVING COUNT(o.id) > 5;
```

```nom
# Nom: compose a user analytics view
system analytics
need database::postgres
need query::users where active=true
flow users->orders->count->filter->response
require freshness<30days
```

### Comparison Table

| Dimension | SQL | Nom |
|-----------|-----|-----|
| **Declarativeness** | What data, not how to fetch | What components, not how to wire |
| **Optimizer** | Query planner (cost-based) | Composition engine (score-based) |
| **Domain** | Data querying and manipulation only | General-purpose application composition |
| **Standardization** | ANSI SQL standard (40+ years) | No standard, single implementation |
| **Tooling** | Every database, every IDE, every cloud | CLI only |
| **Composition** | Views, CTEs, subqueries | Flows, systems, graphs |

### Verdict

```
SQL IS A GOOD ANALOGY: Both are declarative. Both have an optimizer.
                       Both let the engine choose execution strategy.

THE DIFFERENCE:        SQL composes data queries. Nom composes software
                       components. Different domains, same philosophy.
```

---

## 9. Nom vs. Terraform / Infrastructure-as-Code

### The Parallel

Terraform and Nom both declare desired state and let a planner figure out execution.

```hcl
# Terraform: declare infrastructure
resource "aws_instance" "auth" {
  ami           = "ami-0c55b159cbfafe1f0"
  instance_type = "t3.micro"
  tags = { Name = "auth-service" }
}

resource "aws_elasticache_cluster" "redis" {
  cluster_id           = "auth-cache"
  engine               = "redis"
  node_type            = "cache.t3.micro"
  num_cache_nodes      = 1
}
```

```nom
# Nom: declare the application that runs on that infrastructure
system auth
need hash::argon2 where security>0.9
need store::redis where reliability>0.8
flow request->hash->store->response
```

### Comparison Table

| Dimension | Terraform | Nom |
|-----------|-----------|-----|
| **Declares** | Infrastructure resources | Application components |
| **Planner** | Resource dependency graph | Composition graph |
| **State** | terraform.tfstate (tracks real infra) | nom.lock (tracks .nomtu hashes) |
| **Provider ecosystem** | 4,000+ providers (AWS, GCP, Azure, etc.) | 0 .nomtu |
| **Drift detection** | Yes (plan shows changes) | Glass box reports (show what changed) |

### Verdict

```
THEY'RE COMPLEMENTARY: Terraform declares WHERE to run.
                       Nom declares WHAT to run.
                       Together: full-stack declarative composition.
```

---

## 10. Cross-Language Summary Matrix

| Feature | Nom | Rust | Python | Go | TypeScript | Java | Haskell |
|---------|-----|------|--------|----|------------|------|---------|
| **Paradigm** | Compositional-semantic | Multi-paradigm | Multi-paradigm | Imperative | Multi-paradigm | OOP | Functional |
| **Typing** | Contracts | Static, strong | Dynamic | Static, strong | Structural | Nominal | Static, strong |
| **Memory** | Engine-managed (arena/pool) | Ownership + borrowing | GC | GC | GC (JS engine) | GC (JVM) | GC (lazy) |
| **Compilation** | AOT (via LLVM) | AOT (via LLVM) | Interpreted | AOT | JIT (V8/Bun) | JIT (JVM) | AOT/interpreted |
| **Concurrency** | Auto from flow topology | async/await, threads | asyncio, GIL | Goroutines | async/await | Threads, virtual threads | STM, async |
| **Package count** | 0 | 165K | 320K | 350K | 3.2M | 500K | 17K |
| **Supply chain safety** | Content-addressed + scored | Cargo audit | pip-audit (optional) | sumdb | npm audit (limited) | Dependabot | cabal (limited) |
| **Effect tracking** | Built-in (good/bad valence) | None | None | None | None | Checked exceptions (mostly abandoned) | Monads |
| **Readability** | English sentences | Medium | High | High | Medium | Low (verbose) | Low (dense) |
| **Learning curve** | Low (intent) | High (ownership) | Low | Low | Medium | Medium | Very high |
| **Maturity** | Year 0 | 10+ years | 35+ years | 15+ years | 12+ years | 30+ years | 35+ years |

---

## 11. What Nom Does That No Other Language Does

These are capabilities that **cannot be retrofitted** into existing languages
without breaking backward compatibility:

| Capability | Why It's Unique | Closest Competitor |
|-----------|----------------|-------------------|
| **Scored dependencies** | Every .nomtu has 8 quality scores (security, reliability, etc.) | None — npm/pip/cargo have download counts, not quality scores |
| **Effect valence** | Distinguishes good effects (`duoc`) from bad effects (`bi`) | Haskell has monads but no positive/negative distinction |
| **Contract composition** | Contracts verified across component boundaries automatically | Rust traits check locally, not across composition boundaries |
| **Glass box reports** | Full audit trail: what was selected, why, alternatives considered | No language provides this |
| **Classifier-based syntax** | Vietnamese-inspired classifiers give semantic context to every declaration | No language uses linguistic classifiers |
| **Dictionary-grounded composition** | Select from verified atoms, can't fabricate | Unison has content-addressing but no contracts/scores |
| **Writing-style syntax** | Programs read like English sentences, no ceremony | AppleScript tried this but with traditional imperative semantics |

---

## 12. Where Every Language Beats Nom Today

This is the most important section. Every comparison above shares one truth:

```
EVERY LANGUAGE LISTED HAS:
    ✓ A working ecosystem with real packages
    ✓ Real users building real production software
    ✓ IDE support, debuggers, profilers
    ✓ Documentation, tutorials, books
    ✓ Community, conferences, jobs
    ✓ Years or decades of battle-testing

NOM HAS:
    ✓ A compiler that produces native binaries (42 tests passing)
    ✓ A well-researched language design
    ✓ Strong ideas about composition, contracts, and supply chain safety
    ✗ Zero published .nomtu in the dictionary
    ✗ No IDE integration beyond syntax highlighting
    ✗ No debugger
    ✗ No users
    ✗ No production deployments
```

### The Path Forward

Nom's advantages are **architectural** — they come from design decisions that
existing languages cannot adopt without breaking backward compatibility.

But architectural advantages don't matter without:
1. A populated dictionary (at least 1,000 .nomtu for real tasks)
2. Tooling that matches developer expectations (LSP, debugger, error messages)
3. A killer app that demonstrates why composition beats generation
4. Time — every successful language needed 5-10 years to prove itself

```
The ideas are strong. The execution is at the beginning.
Every comparison in this document is theoretical until the dictionary exists.
Phase 0: make it work. Phase 1: make it useful. Phase 2: make it fast.
Then these comparisons become real.
```
