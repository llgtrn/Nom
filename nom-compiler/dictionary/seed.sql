-- Nom Dictionary Seed Data
-- Inserts into the resolver's `nomtu` table.
-- Schema: word, variant, hash, describe, kind,
--         input_type, output_type, effects,
--         pre, post,
--         security, performance, quality, reliability,
--         language,
--         body, signature,
--         is_canonical
-- Primary key: UNIQUE(word, variant, language)

-- ============================================================================
-- CRYPTO (14 words)
-- ============================================================================

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('hash', 'argon2', 'cr_hash_argon2_00000000000000000001', 'convert data into irreversible fixed-length string to protect passwords and verify integrity', 'crypto', 'bytes', 'hashbytes', '["cpu"]', NULL, NULL, 0.96, 0.72, 0.88, 0.95, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('hash', 'sha256', 'cr_hash_sha256_00000000000000000002', 'convert data into irreversible fixed-length string for checksums and verification', 'crypto', 'bytes', 'hashbytes', '["cpu"]', NULL, NULL, 0.91, 0.95, 0.94, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('hash', 'sha512', 'cr_hash_sha512_00000000000000000003', 'compute sha-512 digest for high-security integrity verification', 'crypto', 'bytes', 'hashbytes', '["cpu"]', NULL, NULL, 0.93, 0.93, 0.94, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('hash', 'blake3', 'cr_hash_blake3_00000000000000000004', 'compute blake3 hash with parallel tree hashing for maximum throughput', 'crypto', 'bytes', 'hashbytes', '["cpu"]', NULL, NULL, 0.94, 0.99, 0.97, 0.98, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('encrypt', 'aes256', 'cr_encrypt_aes256_000000000000000005', 'encrypt data using aes-256-gcm authenticated encryption', 'crypto', 'bytes', 'ciphertext', '["cpu"]', NULL, NULL, 0.97, 0.94, 0.96, 0.98, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('encrypt', 'chacha20', 'cr_encrypt_chach_0000000000000000006', 'encrypt data using chacha20-poly1305 stream cipher', 'crypto', 'bytes', 'ciphertext', '["cpu"]', NULL, NULL, 0.96, 0.96, 0.96, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('decrypt', 'aes256', 'cr_decrypt_aes256_000000000000000007', 'decrypt aes-256-gcm authenticated ciphertext back to plaintext', 'crypto', 'ciphertext', 'bytes', '["cpu"]', NULL, NULL, 0.97, 0.94, 0.96, 0.98, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('decrypt', 'chacha20', 'cr_decrypt_chach_0000000000000000008', 'decrypt chacha20-poly1305 ciphertext back to plaintext', 'crypto', 'ciphertext', 'bytes', '["cpu"]', NULL, NULL, 0.96, 0.96, 0.96, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('sign', 'ed25519', 'cr_sign_ed25519_00000000000000000009', 'create digital signature using ed25519 elliptic curve cryptography', 'crypto', 'bytes', 'signature', '["cpu"]', NULL, NULL, 0.97, 0.97, 0.98, 0.99, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('sign', 'rsa', 'cr_sign_rsa_000000000000000000000010', 'create digital signature using rsa-pss with sha-256', 'crypto', 'bytes', 'signature', '["cpu"]', NULL, NULL, 0.93, 0.82, 0.90, 0.96, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('verify', 'ed25519', 'cr_verify_ed2551_000000000000000000011', 'verify ed25519 digital signature against public key', 'crypto', 'signature', 'bool', '["cpu"]', NULL, NULL, 0.97, 0.97, 0.98, 0.99, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('verify', 'rsa', 'cr_verify_rsa_0000000000000000000012', 'verify rsa digital signature against public key', 'crypto', 'signature', 'bool', '["cpu"]', NULL, NULL, 0.93, 0.82, 0.90, 0.96, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('tls', 'rustls', 'cr_tls_rustls_0000000000000000000013', 'establish tls encrypted connection using pure-rust implementation', 'crypto', 'config', 'connection', '["network"]', NULL, NULL, 0.95, 0.93, 0.95, 0.96, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('random', NULL, 'cr_random_000000000000000000000014', 'generate cryptographically secure random bytes', 'crypto', 'int', 'bytes', '[]', NULL, NULL, 0.95, 0.98, 0.97, 0.99, 'rust', NULL, NULL, 1);

-- ============================================================================
-- DATA (14 words)
-- ============================================================================

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('store', 'redis', 'da_store_redis_0000000000000000000001', 'save and retrieve data persistently using redis key-value database', 'data', 'any', 'bool', '["database", "network"]', NULL, NULL, 0.88, 0.92, 0.91, 0.93, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('store', 'postgres', 'da_store_postg_0000000000000000000002', 'query and store structured data in postgresql relational database', 'data', 'any', 'bool', '["database", "network"]', NULL, NULL, 0.92, 0.88, 0.92, 0.96, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('store', 'sqlite', 'da_store_sqlit_0000000000000000000003', 'store and query data in embedded sqlite database file', 'data', 'any', 'bool', '["database"]', NULL, NULL, 0.85, 0.90, 0.90, 0.95, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('store', 'memory', 'da_store_memor_0000000000000000000004', 'save and retrieve data in process memory for testing and prototyping', 'data', 'any', 'bool', '[]', NULL, NULL, 0.70, 0.99, 0.76, 0.60, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('store', 's3', 'da_store_s3_00000000000000000000000005', 'upload and download objects from s3-compatible object storage', 'data', 'bytes', 'url', '["network"]', NULL, NULL, 0.90, 0.85, 0.90, 0.94, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('cache', 'lru', 'da_cache_lru_00000000000000000000006', 'store recently used values in bounded least-recently-used cache', 'data', 'any', 'option', '[]', NULL, NULL, 0.80, 0.98, 0.91, 0.95, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('cache', 'writethrough', 'da_cache_write_0000000000000000000007', 'cache with synchronous write-through to backing store on every set', 'data', 'any', 'bool', '["database"]', NULL, NULL, 0.82, 0.88, 0.88, 0.93, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('queue', 'kafka', 'da_queue_kafka_0000000000000000000008', 'publish and consume messages from apache kafka distributed log', 'data', 'message', 'ack', '["network"]', NULL, NULL, 0.88, 0.91, 0.90, 0.94, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('queue', 'rabbitmq', 'da_queue_rabbi_0000000000000000000009', 'publish and consume messages from rabbitmq message broker', 'data', 'message', 'ack', '["network"]', NULL, NULL, 0.87, 0.89, 0.90, 0.93, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('queue', 'channel', 'da_queue_chann_0000000000000000000010', 'send and receive messages through in-process async channel', 'data', 'any', 'any', '[]', NULL, NULL, 0.80, 0.99, 0.92, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('search', 'fulltext', 'da_search_fullt_000000000000000000011', 'index and search text documents using inverted index full-text search', 'data', 'text', 'results', '["cpu"]', NULL, NULL, 0.80, 0.88, 0.87, 0.92, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('search', 'vector', 'da_search_vecto_000000000000000000012', 'find nearest neighbors in high-dimensional vector space', 'data', 'vector', 'results', '["cpu"]', NULL, NULL, 0.78, 0.86, 0.84, 0.90, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('migrate', NULL, 'da_migrate_00000000000000000000000013', 'run database schema migrations forward or backward', 'data', 'config', 'bool', '["database"]', NULL, NULL, 0.85, 0.80, 0.85, 0.90, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('transaction', NULL, 'da_transaction_0000000000000000000014', 'execute multiple database operations atomically with rollback on failure', 'data', 'query', 'result', '["database"]', NULL, NULL, 0.90, 0.85, 0.90, 0.96, 'rust', NULL, NULL, 1);

-- ============================================================================
-- NETWORK (14 words)
-- ============================================================================

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('http', 'server', 'ne_http_server_0000000000000000000001', 'serve web content and handle http requests on a network port', 'network', 'config', 'server', '["network"]', NULL, NULL, 0.90, 0.94, 0.93, 0.95, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('http', 'client', 'ne_http_client_0000000000000000000002', 'send http requests to remote servers and receive responses', 'network', 'request', 'response', '["network"]', NULL, NULL, 0.88, 0.93, 0.91, 0.94, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('http', 'route', 'ne_http_route_00000000000000000000003', 'define url path pattern to handler function mapping for http router', 'network', 'pattern', 'handler', '[]', NULL, NULL, 0.88, 0.96, 0.93, 0.95, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('websocket', NULL, 'ne_websocket_000000000000000000000004', 'establish persistent bidirectional communication channel over websocket', 'network', 'url', 'connection', '["network"]', NULL, NULL, 0.86, 0.91, 0.90, 0.92, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('grpc', 'server', 'ne_grpc_server_0000000000000000000005', 'serve remote procedure calls using grpc protocol with protobuf', 'network', 'config', 'server', '["network"]', NULL, NULL, 0.90, 0.95, 0.93, 0.94, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('grpc', 'client', 'ne_grpc_client_0000000000000000000006', 'call remote grpc services with protobuf serialization', 'network', 'request', 'response', '["network"]', NULL, NULL, 0.90, 0.95, 0.93, 0.94, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('dns', NULL, 'ne_dns_0000000000000000000000000000007', 'resolve domain names to ip addresses via dns lookup', 'network', 'text', 'ip', '["network"]', NULL, NULL, 0.82, 0.90, 0.87, 0.88, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('proxy', NULL, 'ne_proxy_000000000000000000000000008', 'forward network traffic between client and upstream server through proxy', 'network', 'request', 'response', '["network"]', NULL, NULL, 0.85, 0.90, 0.89, 0.91, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('loadbalancer', NULL, 'ne_loadbalance_0000000000000000000009', 'distribute incoming requests across multiple backend servers', 'network', 'request', 'response', '["network"]', NULL, NULL, 0.87, 0.92, 0.91, 0.93, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('limiter', 'tokenbucket', 'ne_limiter_toke_000000000000000000010', 'control rate of incoming requests using token bucket algorithm', 'network', 'request', 'request', '["network"]', NULL, NULL, 0.85, 0.95, 0.91, 0.92, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('limiter', 'leakybucket', 'ne_limiter_leak_000000000000000000011', 'control rate of incoming requests using leaky bucket algorithm', 'network', 'request', 'request', '["network"]', NULL, NULL, 0.85, 0.94, 0.90, 0.92, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('cors', NULL, 'ne_cors_0000000000000000000000000012', 'add cross-origin resource sharing headers to http responses', 'network', 'config', 'middleware', '[]', NULL, NULL, 0.85, 0.99, 0.94, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('sse', NULL, 'ne_sse_00000000000000000000000000013', 'stream server-sent events to connected clients over http', 'network', 'event', 'stream', '["network"]', NULL, NULL, 0.84, 0.92, 0.90, 0.91, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('tcp', NULL, 'ne_tcp_00000000000000000000000000014', 'open and manage raw tcp socket connections', 'network', 'address', 'connection', '["network"]', NULL, NULL, 0.82, 0.95, 0.90, 0.94, 'rust', NULL, NULL, 1);

-- ============================================================================
-- IO (8 words)
-- ============================================================================

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('print', 'stdout', 'io_print_stdout_000000000000000000001', 'output text to standard output stream for display', 'io', 'text', 'void', '["stdout"]', NULL, NULL, 1.0, 1.0, 1.0, 1.0, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('print', 'stderr', 'io_print_stderr_000000000000000000002', 'output text to standard error stream for diagnostics', 'io', 'text', 'void', '["stderr"]', NULL, NULL, 1.0, 1.0, 1.0, 1.0, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('read', 'stdin', 'io_read_stdin_00000000000000000000003', 'read text input from standard input stream', 'io', 'void', 'text', '["stdin"]', NULL, NULL, 0.90, 0.95, 0.93, 0.95, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('read', 'file', 'io_read_file_000000000000000000000004', 'read entire contents of a file from filesystem path', 'io', 'path', 'bytes', '["filesystem"]', NULL, NULL, 0.85, 0.92, 0.90, 0.94, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('write', 'file', 'io_write_file_00000000000000000000005', 'write data to a file at filesystem path creating or overwriting', 'io', 'bytes', 'bool', '["filesystem"]', NULL, NULL, 0.83, 0.91, 0.89, 0.93, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('log', 'structured', 'io_log_structur_000000000000000000006', 'emit structured log records with level and key-value fields', 'io', 'text', 'void', '["stdout"]', NULL, NULL, 0.90, 0.97, 0.95, 0.98, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('log', 'file', 'io_log_file_0000000000000000000000007', 'append log records to a rolling file on disk', 'io', 'text', 'void', '["filesystem"]', NULL, NULL, 0.88, 0.93, 0.92, 0.95, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('env', NULL, 'io_env_000000000000000000000000000008', 'read environment variable value by name', 'io', 'text', 'option', '[]', NULL, NULL, 0.75, 1.0, 0.91, 0.99, 'rust', NULL, NULL, 1);

-- ============================================================================
-- AUTH (8 words)
-- ============================================================================

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('auth', 'jwt', 'au_auth_jwt_0000000000000000000000001', 'verify identity of user through json web token authentication', 'auth', 'token', 'claims', '["cpu"]', NULL, NULL, 0.89, 0.96, 0.92, 0.91, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('auth', 'oauth', 'au_auth_oauth_00000000000000000000002', 'authenticate user via oauth2 authorization code flow', 'auth', 'config', 'token', '["network"]', NULL, NULL, 0.91, 0.80, 0.87, 0.88, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('auth', 'session', 'au_auth_sessio_00000000000000000000003', 'manage user sessions with server-side cookie-based authentication', 'auth', 'request', 'session', '["database"]', NULL, NULL, 0.86, 0.92, 0.89, 0.90, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('auth', 'apikey', 'au_auth_apikey_00000000000000000000004', 'authenticate api requests using bearer token or header api key', 'auth', 'request', 'identity', '["cpu"]', NULL, NULL, 0.82, 0.99, 0.91, 0.94, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('totp', NULL, 'au_totp_0000000000000000000000000000005', 'generate and verify time-based one-time passwords for two-factor auth', 'auth', 'secret', 'code', '["cpu"]', NULL, NULL, 0.92, 0.99, 0.95, 0.95, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('authorize', NULL, 'au_authorize_000000000000000000000006', 'check if authenticated identity has permission to perform action', 'auth', 'identity', 'bool', '["cpu"]', NULL, NULL, 0.90, 0.98, 0.94, 0.95, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('hash', 'bcrypt', 'au_hash_bcrypt_00000000000000000000007', 'hash password using bcrypt adaptive cost function', 'auth', 'text', 'hashbytes', '["cpu"]', NULL, NULL, 0.90, 0.65, 0.83, 0.94, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('validate', 'email', 'au_validate_ema_00000000000000000000008', 'validate that a string is a well-formed email address', 'auth', 'text', 'bool', '[]', NULL, NULL, 0.80, 1.0, 0.91, 0.95, 'rust', NULL, NULL, 1);

-- ============================================================================
-- COMPUTE (12 words)
-- ============================================================================

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('sort', NULL, 'co_sort_0000000000000000000000000000001', 'sort a collection of elements in ascending order', 'compute', 'list', 'list', '["cpu"]', NULL, NULL, 1.0, 0.95, 0.98, 1.0, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('filter', NULL, 'co_filter_000000000000000000000000002', 'retain only elements matching a predicate from a collection', 'compute', 'list', 'list', '[]', NULL, NULL, 1.0, 0.96, 0.99, 1.0, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('map', NULL, 'co_map_00000000000000000000000000000003', 'transform each element in a collection by applying a function', 'compute', 'list', 'list', '[]', NULL, NULL, 1.0, 0.96, 0.99, 1.0, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('reduce', NULL, 'co_reduce_000000000000000000000000004', 'combine all elements of a collection into a single accumulated value', 'compute', 'list', 'any', '[]', NULL, NULL, 1.0, 0.94, 0.98, 1.0, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('compress', 'gzip', 'co_compress_gzi_00000000000000000005', 'compress data using gzip deflate algorithm', 'compute', 'bytes', 'bytes', '["cpu"]', NULL, NULL, 0.85, 0.88, 0.90, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('compress', 'zstd', 'co_compress_zst_00000000000000000006', 'compress data using zstandard algorithm for high ratio and speed', 'compute', 'bytes', 'bytes', '["cpu"]', NULL, NULL, 0.85, 0.96, 0.93, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('decompress', 'gzip', 'co_decompress_g_00000000000000000007', 'decompress gzip-compressed data back to original bytes', 'compute', 'bytes', 'bytes', '["cpu"]', NULL, NULL, 0.85, 0.88, 0.90, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('decompress', 'zstd', 'co_decompress_z_00000000000000000008', 'decompress zstandard-compressed data back to original bytes', 'compute', 'bytes', 'bytes', '["cpu"]', NULL, NULL, 0.85, 0.96, 0.93, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('serialize', 'json', 'co_serialize_js_00000000000000000009', 'convert structured data into json text representation', 'compute', 'any', 'text', '[]', NULL, NULL, 0.90, 0.94, 0.94, 0.99, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('serialize', 'protobuf', 'co_serialize_pr_00000000000000000010', 'convert structured data into protocol buffers binary format', 'compute', 'any', 'bytes', '[]', NULL, NULL, 0.90, 0.97, 0.95, 0.98, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('deserialize', 'json', 'co_deserialize__00000000000000000011', 'parse json text into structured data', 'compute', 'text', 'any', '[]', NULL, NULL, 0.88, 0.94, 0.93, 0.98, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('deserialize', 'protobuf', 'co_deserialize__00000000000000000012', 'parse protocol buffers binary data into structured values', 'compute', 'bytes', 'any', '[]', NULL, NULL, 0.88, 0.97, 0.94, 0.98, 'rust', NULL, NULL, 1);

-- ============================================================================
-- OS (12 words)
-- ============================================================================

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('process', 'spawn', 'os_process_spaw_00000000000000000001', 'spawn a child process and optionally capture its output', 'os', 'command', 'handle', '["process"]', NULL, NULL, 0.75, 0.90, 0.86, 0.93, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('process', 'kill', 'os_process_kill_00000000000000000002', 'send termination signal to a running process by handle or pid', 'os', 'handle', 'bool', '["process"]', NULL, NULL, 0.70, 0.95, 0.85, 0.90, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('thread', 'spawn', 'os_thread_spawn_00000000000000000003', 'spawn an os-level thread to run a function concurrently', 'os', 'function', 'handle', '[]', NULL, NULL, 0.80, 0.95, 0.90, 0.96, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('signal', NULL, 'os_signal_000000000000000000000000004', 'register handler for unix signals like sigint sigterm sighup', 'os', 'int', 'stream', '["process"]', NULL, NULL, 0.78, 0.95, 0.87, 0.90, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('pipe', NULL, 'os_pipe_00000000000000000000000000005', 'create a unidirectional byte stream between two processes or tasks', 'os', 'void', 'pipe', '["process"]', NULL, NULL, 0.80, 0.94, 0.89, 0.93, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('socket', 'unix', 'os_socket_unix_00000000000000000000006', 'create a unix domain socket for local inter-process communication', 'os', 'path', 'connection', '["filesystem"]', NULL, NULL, 0.82, 0.96, 0.91, 0.94, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('mount', NULL, 'os_mount_0000000000000000000000000007', 'mount a filesystem at a specified path in the directory tree', 'os', 'config', 'bool', '["filesystem", "process"]', NULL, NULL, 0.72, 0.90, 0.83, 0.88, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('cgroup', NULL, 'os_cgroup_000000000000000000000000008', 'create or configure a linux cgroup to limit process resource usage', 'os', 'config', 'handle', '["process"]', NULL, NULL, 0.78, 0.88, 0.84, 0.85, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('namespace', NULL, 'os_namespace_000000000000000000000009', 'create a linux namespace for process isolation', 'os', 'config', 'handle', '["process"]', NULL, NULL, 0.80, 0.85, 0.83, 0.84, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('timer', NULL, 'os_timer_0000000000000000000000000010', 'create a one-shot or recurring timer that fires after a duration', 'os', 'duration', 'event', '[]', NULL, NULL, 0.90, 0.99, 0.96, 0.98, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('sleep', NULL, 'os_sleep_0000000000000000000000000011', 'suspend current task or thread for a specified duration', 'os', 'duration', 'void', '[]', NULL, NULL, 1.0, 1.0, 1.0, 1.0, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('mkdir', NULL, 'os_mkdir_0000000000000000000000000012', 'create a directory and any missing parent directories on the filesystem', 'os', 'path', 'bool', '["filesystem"]', NULL, NULL, 0.85, 0.98, 0.93, 0.97, 'rust', NULL, NULL, 1);

-- ============================================================================
-- AI / ML (8 words)
-- ============================================================================

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('embed', NULL, 'ai_embed_0000000000000000000000000001', 'convert text into dense vector embedding for semantic similarity', 'ai', 'text', 'vector', '["cpu", "network"]', NULL, NULL, 0.82, 0.78, 0.83, 0.88, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('classify', NULL, 'ai_classify_0000000000000000000000002', 'assign input to one of a set of predefined categories', 'ai', 'text', 'label', '["cpu"]', NULL, NULL, 0.80, 0.80, 0.82, 0.85, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('generate', 'text', 'ai_generate_tex_00000000000000000003', 'generate natural language text from a prompt using language model', 'ai', 'text', 'text', '["cpu", "network"]', NULL, NULL, 0.75, 0.60, 0.72, 0.82, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('generate', 'code', 'ai_generate_cod_00000000000000000004', 'generate source code from a natural language specification', 'ai', 'text', 'text', '["cpu", "network"]', NULL, NULL, 0.70, 0.55, 0.68, 0.78, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('tokenize', NULL, 'ai_tokenize_0000000000000000000000005', 'split text into subword tokens for language model input', 'ai', 'text', 'list', '["cpu"]', NULL, NULL, 0.90, 0.95, 0.94, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('vector', 'cosine', 'ai_vector_cosin_00000000000000000006', 'compute cosine similarity between two vectors', 'ai', 'vector', 'float', '[]', NULL, NULL, 0.90, 0.99, 0.96, 1.0, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('vector', 'dot', 'ai_vector_dot_000000000000000000000007', 'compute dot product between two vectors', 'ai', 'vector', 'float', '[]', NULL, NULL, 0.90, 0.99, 0.96, 1.0, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('vector', 'normalize', 'ai_vector_norma_00000000000000000008', 'normalize a vector to unit length', 'ai', 'vector', 'vector', '[]', NULL, NULL, 0.90, 0.99, 0.96, 1.0, 'rust', NULL, NULL, 1);

-- ============================================================================
-- GRAPH (7 words)
-- ============================================================================

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('node', NULL, 'gr_node_00000000000000000000000000001', 'create a node in a directed or undirected graph structure', 'graph', 'any', 'handle', '[]', NULL, NULL, 0.90, 0.95, 0.94, 0.98, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('edge', NULL, 'gr_edge_00000000000000000000000000002', 'create a weighted or unweighted edge between two graph nodes', 'graph', 'handle', 'handle', '[]', NULL, NULL, 0.90, 0.95, 0.94, 0.98, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('traverse', 'bfs', 'gr_traverse_bfs_00000000000000000003', 'visit all reachable graph nodes in breadth-first order', 'graph', 'handle', 'list', '["cpu"]', NULL, NULL, 0.90, 0.90, 0.93, 0.98, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('traverse', 'dfs', 'gr_traverse_dfs_00000000000000000004', 'visit all reachable graph nodes in depth-first order', 'graph', 'handle', 'list', '["cpu"]', NULL, NULL, 0.90, 0.90, 0.93, 0.98, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('shortest_path', NULL, 'gr_shortest_pat_00000000000000000005', 'find shortest path between two nodes using dijkstra algorithm', 'graph', 'handle', 'list', '["cpu"]', NULL, NULL, 0.90, 0.88, 0.92, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('connected', NULL, 'gr_connected_000000000000000000000006', 'test whether two nodes are connected in a graph', 'graph', 'handle', 'bool', '["cpu"]', NULL, NULL, 0.90, 0.92, 0.93, 0.98, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('topological_sort', NULL, 'gr_topological__00000000000000000007', 'order directed acyclic graph nodes so every edge points forward', 'graph', 'handle', 'list', '["cpu"]', NULL, NULL, 0.90, 0.90, 0.93, 0.98, 'rust', NULL, NULL, 1);

-- ============================================================================
-- AGENT (8 words)
-- ============================================================================

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('agent', 'supervisor', 'ag_agent_super_00000000000000000001', 'spawn a supervisor agent that manages child worker lifecycles', 'agent', 'config', 'handle', '["process"]', NULL, NULL, 0.85, 0.88, 0.88, 0.92, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('agent', 'worker', 'ag_agent_worke_00000000000000000002', 'spawn a worker agent that processes tasks from a queue', 'agent', 'config', 'handle', '["process"]', NULL, NULL, 0.85, 0.90, 0.89, 0.93, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('message', 'send', 'ag_message_send_0000000000000000003', 'send a typed message to an agent or channel by handle', 'agent', 'any', 'bool', '[]', NULL, NULL, 0.88, 0.97, 0.93, 0.96, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('message', 'recv', 'ag_message_recv_0000000000000000004', 'receive the next message from an agent inbox or channel', 'agent', 'handle', 'any', '[]', NULL, NULL, 0.88, 0.97, 0.93, 0.96, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('channel', 'mpsc', 'ag_channel_mpsc_0000000000000000005', 'create multi-producer single-consumer async channel', 'agent', 'int', 'channel', '[]', NULL, NULL, 0.88, 0.98, 0.94, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('channel', 'broadcast', 'ag_channel_broa_0000000000000000006', 'create broadcast channel where all receivers get every message', 'agent', 'int', 'channel', '[]', NULL, NULL, 0.88, 0.95, 0.93, 0.95, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('schedule', 'cron', 'ag_schedule_cro_0000000000000000007', 'schedule a task to run on a cron expression schedule', 'agent', 'text', 'handle', '["process"]', NULL, NULL, 0.82, 0.90, 0.87, 0.90, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('observe', 'metrics', 'ag_observe_metr_0000000000000000008', 'collect and expose runtime metrics for monitoring and alerting', 'agent', 'config', 'void', '["network"]', NULL, NULL, 0.85, 0.95, 0.91, 0.94, 'rust', NULL, NULL, 1);

-- ============================================================================
-- CONCURRENCY (6 words)
-- ============================================================================

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('spawn', NULL, 'cc_spawn_0000000000000000000000000001', 'spawn an async task on the runtime executor', 'concurrency', 'function', 'handle', '[]', NULL, NULL, 0.88, 0.97, 0.94, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('join', NULL, 'cc_join_00000000000000000000000000002', 'wait for multiple async tasks to complete and collect results', 'concurrency', 'list', 'list', '[]', NULL, NULL, 0.90, 0.96, 0.94, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('select', NULL, 'cc_select_000000000000000000000000003', 'wait for first of multiple async operations to complete', 'concurrency', 'list', 'any', '[]', NULL, NULL, 0.88, 0.96, 0.92, 0.95, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('mutex', NULL, 'cc_mutex_0000000000000000000000000004', 'create a mutual exclusion lock for shared mutable state', 'concurrency', 'any', 'guard', '[]', NULL, NULL, 0.85, 0.92, 0.90, 0.96, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('semaphore', NULL, 'cc_semaphore_000000000000000000000005', 'limit concurrent access to a resource with counting semaphore', 'concurrency', 'int', 'handle', '[]', NULL, NULL, 0.86, 0.94, 0.91, 0.96, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('retry', NULL, 'cc_retry_0000000000000000000000000006', 'retry a fallible operation with configurable backoff strategy', 'concurrency', 'function', 'any', '[]', NULL, NULL, 0.85, 0.85, 0.87, 0.92, 'rust', NULL, NULL, 1);

-- ============================================================================
-- TEMPLATE / FORMAT (5 words)
-- ============================================================================

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('template', 'html', 'tf_template_htm_00000000000000000001', 'render html from a template with variable substitution', 'template', 'any', 'text', '[]', NULL, NULL, 0.82, 0.93, 0.91, 0.96, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('template', 'text', 'tf_template_tex_00000000000000000002', 'render plain text from a template string with variable substitution', 'template', 'any', 'text', '[]', NULL, NULL, 0.85, 0.97, 0.93, 0.98, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('format', 'json', 'tf_format_json_000000000000000000003', 'pretty-print a json value with indentation', 'template', 'any', 'text', '[]', NULL, NULL, 0.90, 0.95, 0.95, 0.99, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('format', 'csv', 'tf_format_csv_0000000000000000000004', 'format structured data as comma-separated values text', 'template', 'list', 'text', '[]', NULL, NULL, 0.85, 0.94, 0.92, 0.97, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('parse', 'json', 'tf_parse_json_0000000000000000000005', 'parse json text into a structured value', 'template', 'text', 'any', '[]', NULL, NULL, 0.88, 0.94, 0.93, 0.98, 'rust', NULL, NULL, 1);

-- ============================================================================
-- TESTING (4 words)
-- ============================================================================

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('assert', NULL, 'te_assert_000000000000000000000000001', 'assert that a boolean condition is true or panic with message', 'testing', 'bool', 'void', '[]', NULL, NULL, 1.0, 1.0, 1.0, 1.0, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('mock', NULL, 'te_mock_00000000000000000000000000002', 'create a mock implementation of a trait or interface for testing', 'testing', 'config', 'any', '[]', NULL, NULL, 0.80, 0.95, 0.88, 0.90, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('bench', NULL, 'te_bench_0000000000000000000000000003', 'measure execution time and throughput of a function', 'testing', 'function', 'report', '["cpu"]', NULL, NULL, 0.90, 0.90, 0.92, 0.95, 'rust', NULL, NULL, 1);

INSERT OR REPLACE INTO nomtu (word, variant, hash, describe, kind, input_type, output_type, effects, pre, post, security, performance, quality, reliability, language, body, signature, is_canonical)
VALUES ('snapshot', NULL, 'te_snapshot_0000000000000000000000004', 'compare function output against stored reference snapshot', 'testing', 'any', 'bool', '["filesystem"]', NULL, NULL, 0.85, 0.92, 0.90, 0.93, 'rust', NULL, NULL, 1);
