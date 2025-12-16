# KStore API Documentation

Complete API reference for the KStore key-value store server.

## Base URL

```
http://127.0.0.1:8080
```

## Response Formats

All endpoints return appropriate HTTP status codes and either plain text or JSON responses.

### Common Status Codes
- `200 OK` - Request succeeded
- `201 Created` - Resource created successfully
- `400 Bad Request` - Invalid input or validation error
- `404 Not Found` - Key or resource not found
- `409 Conflict` - Resource already exists
- `500 Internal Server Error` - Server error

---

## Health & Monitoring Endpoints

### GET /health

Check if the server is running and healthy.

**Response**
```json
{
  "status": "healthy",
  "timestamp": 1702742400
}
```

**Status Codes**
- `200 OK` - Server is healthy

---

### GET /stats

Get comprehensive statistics about the key-value store.

**Response**
```json
{
  "total_keys": 150,
  "total_size_bytes": 524288,
  "operations_count": 1523,
  "uptime_seconds": 3600
}
```

**Fields**
- `total_keys` - Number of keys currently stored
- `total_size_bytes` - Total size of all values in bytes
- `operations_count` - Total number of operations performed
- `uptime_seconds` - Server uptime in seconds

**Status Codes**
- `200 OK` - Statistics retrieved successfully

---

## Key-Value Operations

### GET /kv/

List all keys in the store with optional filtering.

**Query Parameters**
- `prefix` (optional) - Filter keys starting with this prefix
- `limit` (optional) - Maximum number of keys to return

**Examples**
```bash
GET /kv/
GET /kv/?prefix=user
GET /kv/?prefix=session&limit=10
```

**Response**
```json
["key1", "key2", "key3"]
```

**Status Codes**
- `200 OK` - Keys retrieved successfully
- `404 Not Found` - No keys found (returns empty array)

---

### GET /kv/{key}

Retrieve the value associated with a key.

**Path Parameters**
- `key` - The key to retrieve

**Response**
Plain text value

**Status Codes**
- `200 OK` - Value retrieved successfully
- `404 Not Found` - Key does not exist

**Example**
```bash
curl http://127.0.0.1:8080/kv/username
```

---

### GET /kv/{key}/info

Get detailed metadata about a specific key.

**Path Parameters**
- `key` - The key to get information about

**Response**
```json
{
  "key": "username",
  "size": 128,
  "created_at": 1702742400,
  "updated_at": 1702742500,
  "access_count": 42
}
```

**Fields**
- `key` - The key name
- `size` - Value size in bytes
- `created_at` - Unix timestamp of creation
- `updated_at` - Unix timestamp of last update
- `access_count` - Number of times the key has been accessed

**Status Codes**
- `200 OK` - Information retrieved successfully
- `404 Not Found` - Key does not exist

---

### GET /kv/{key}/exists

Check if a key exists without retrieving its value.

**Path Parameters**
- `key` - The key to check

**Response**
```json
{
  "exists": true
}
```

**Status Codes**
- `200 OK` - Check completed (always returns 200)

---

### POST /kv/{key}

Create a new key-value pair. Fails if the key already exists.

**Path Parameters**
- `key` - The key to create

**Request Body**
Plain text value (max 10 MB)

**Status Codes**
- `201 Created` - Key created successfully
- `400 Bad Request` - Validation error (key too long, value too large, etc.)
- `409 Conflict` - Key already exists

**Validation Rules**
- Key must not be empty
- Key must be ≤256 bytes
- Value must be ≤10 MB

**Example**
```bash
curl -X POST -d "John Doe" http://127.0.0.1:8080/kv/username
```

---

### PUT /kv/{key}

Update an existing key-value pair. Fails if the key doesn't exist.

**Path Parameters**
- `key` - The key to update

**Request Body**
Plain text value (max 10 MB)

**Status Codes**
- `200 OK` - Key updated successfully
- `400 Bad Request` - Key does not exist or validation error

**Example**
```bash
curl -X PUT -d "Jane Doe" http://127.0.0.1:8080/kv/username
```

---

### DELETE /kv/{key}

Delete a key-value pair.

**Path Parameters**
- `key` - The key to delete

**Status Codes**
- `200 OK` - Key deleted successfully
- `404 Not Found` - Key does not exist

**Example**
```bash
curl -X DELETE http://127.0.0.1:8080/kv/username
```

---

### DELETE /kv/prefix/{prefix}

Delete all keys that start with a given prefix.

**Path Parameters**
- `prefix` - The prefix to match

**Response**
```json
{
  "deleted_count": 15
}
```

**Status Codes**
- `200 OK` - Deletion completed (even if 0 keys deleted)

**Example**
```bash
curl -X DELETE http://127.0.0.1:8080/kv/prefix/session:
```

---

## Advanced Operations

### GET /kv/r/{regex}

Find all values where the key matches a regular expression pattern.

**Path Parameters**
- `regex` - Regular expression pattern (URL-encoded)

**Response**
```json
["value1", "value2", "value3"]
```

**Status Codes**
- `200 OK` - Search completed successfully
- `400 Bad Request` - Invalid regex pattern
- `404 Not Found` - No matching keys found

**Example**
```bash
curl http://127.0.0.1:8080/kv/r/^user:[0-9]+$
```

---

### POST /batch

Set multiple key-value pairs in a single request.

**Request Body**
```json
[
  {
    "key": "key1",
    "value": "value1"
  },
  {
    "key": "key2",
    "value": "value2"
  }
]
```

**Response**
```json
{
  "success_count": 2
}
```

**Status Codes**
- `200 OK` - Batch operation completed
- `400 Bad Request` - Invalid JSON or validation error

**Notes**
- Failed individual items are skipped, not counted
- All validation rules apply to each item
- Existing keys are overwritten

**Example**
```bash
curl -X POST http://127.0.0.1:8080/batch \
  -H "Content-Type: application/json" \
  -d '[{"key":"k1","value":"v1"},{"key":"k2","value":"v2"}]'
```

---

## Maintenance Operations

### POST /backup

Create a timestamped backup of the entire database.

**Response**
Plain text: "Backup created successfully"

**Status Codes**
- `200 OK` - Backup created successfully
- `500 Internal Server Error` - Backup failed

**Backup File**
Creates file: `kvstore_backup_{timestamp}.db`

**Example**
```bash
curl -X POST http://127.0.0.1:8080/backup
```

---

### POST /compact

Manually trigger database compaction to optimize file size.

**Response**
Plain text: "Database compacted successfully"

**Status Codes**
- `200 OK` - Compaction completed

**Notes**
- Removes deleted key entries from the file
- Briefly blocks all operations
- Recommended after many deletions

**Example**
```bash
curl -X POST http://127.0.0.1:8080/compact
```

---

## Error Responses

All error responses return plain text or JSON with descriptive messages.

**Validation Errors**
```
Key cannot be empty
Key exceeds maximum size of 256 bytes
Value exceeds maximum size of 10485760 bytes
```

**Not Found Errors**
```
Key not found
No values matched the pattern
```

**Conflict Errors**
```
Key already exists
```

**Regex Errors**
```
Invalid regex pattern: <error details>
```

---

## Rate Limiting

No built-in rate limiting. Use a reverse proxy (nginx, traefik) for production deployments.

---

## Authentication

No built-in authentication. The server is intended for:
- Local development
- Deployment behind authenticated reverse proxy
- Trusted network environments

For production use, implement authentication at the proxy level.

---

## Content Types

**Request**
- Plain text for simple values
- `application/json` for batch operations

**Response**
- Plain text for values and simple messages
- `application/json` for structured data (stats, lists, metadata)

---

## Concurrency

All endpoints are thread-safe and support concurrent requests. The server uses Rust's Mutex and Arc for synchronization.

---

## Performance Tips

1. Use batch operations for multiple inserts
2. Use prefix filtering when listing keys
3. Check existence before creating keys to avoid conflicts
4. Compact periodically after many deletions
5. Use GET /kv/{key}/info instead of GET when you only need metadata
6. Monitor /stats endpoint to track store growth
