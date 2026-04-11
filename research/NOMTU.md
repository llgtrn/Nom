# .nomtu — The Dictionary Format

**A .nomtu is a row in a database. Not a file. Not a package.
One row = one word = one concept with scored implementations.**

---

## 1. What a .nomtu Is

```
A .nomtu is a ROW in the nomdict database (nom.dev).
It describes one concept: what it does, what it needs, what it gives.
The actual compiled code is in blob storage, pointed to by impl_hash.
```

## 2. Database Schema

```sql
CREATE TABLE nomdict (
    word        TEXT NOT NULL,
    variant     TEXT,
    kind        TEXT NOT NULL,
    describe    TEXT NOT NULL,        -- English sentence for semantic search
    
    -- contract
    input       TEXT NOT NULL,
    output      TEXT NOT NULL,
    effects     TEXT,
    pre         TEXT,
    post        TEXT,
    
    -- scores (0.0 to 1.0)
    security    REAL,
    performance REAL,
    quality     REAL,
    reliability REAL,
    
    -- provenance
    source      TEXT NOT NULL,
    version     TEXT,
    license     TEXT NOT NULL,
    tests       INTEGER,
    
    -- implementation pointer
    impl_hash   TEXT NOT NULL,        -- content hash → blob storage
    
    PRIMARY KEY (word, variant)
);
```

## 3. Example Rows

```
word=hash      variant=argon2  kind=crypto  describe="convert data into irreversible string to protect passwords"
               input=bytes  output=hashbytes  effects=cpu  security=0.96  performance=0.72
               source=rustcrypto  license=MIT  tests=847  impl_hash=a3f71b02...

word=hash      variant=bcrypt  kind=crypto  describe="convert data into irreversible string to protect passwords"
               input=bytes  output=hashbytes  effects=cpu  security=0.89  performance=0.81
               source=bcryptrs  license=MIT  tests=523  impl_hash=c4d82e35...

word=store     variant=redis   kind=data    describe="save and retrieve data persistently"
               input=any  output=bool  effects=db,net  reliability=0.93
               source=redis  license=BSD  tests=1205  impl_hash=b4e82c13...

word=print     variant=stdout  kind=io      describe="output text to screen"
               input=text  output=void  effects=stdout  quality=1.0
               source=libc  license=MIT  tests=50  impl_hash=e7h15f46...
```

## 4. How the Compiler Uses It

```
.nom sentence:    need hash::argon2 where security>0.9

Compiler query:   SELECT * FROM nomdict
                  WHERE word='hash' AND variant='argon2'

Result:           one row with contract + impl_hash

Check:            security 0.96 > 0.9? ✓

Fetch impl:       GET nom.dev/blobs/a3f71b02 → argon2.wasm
                  (cached locally after first download)

Compile:          LLVM turns argon2.wasm → native machine code
```

## 5. Semantic Search via `describe`

When the compiler doesn't find a word by name, it searches descriptions:

```
.nom sentence:    need something to protect passwords

Compiler query:   SELECT * FROM nomdict
                  WHERE describe @@ 'protect & passwords'
                  ORDER BY security DESC LIMIT 1

Result:           word=hash, variant=argon2 (best security score)
```

## 6. Where Everything Lives

```
nom.dev           PostgreSQL + blob storage (online, millions of rows)
~/.nom/nomdict.db  SQLite cache (local, frequently used rows)
~/.nom/cache/      Downloaded .wasm blobs (local, content-addressed)
./nom.lock         Pinned word+variant+impl_hash (reproducible builds)
```

## 7. Content Addressing

The `impl_hash` is a SHA-256 of the compiled WASM blob.
Same code = same hash = same identity forever.
Can't tamper (changing code changes hash).
Can't unpublish (hash still valid, blob still in storage).
Reproducible (nom.lock pins exact hashes).
