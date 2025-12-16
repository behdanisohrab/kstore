# Changelog

All notable changes to the KStore project will be documented in this file.

## [0.2.0] - 2024-12-16

### Added

#### Metadata & Analytics
- **Key Metadata Tracking**: Each key now stores creation timestamp, last update timestamp, and access count
- **Statistics Endpoint** (`GET /stats`): Returns comprehensive store statistics including total keys, total size, operations count, and server uptime
- **Key Info Endpoint** (`GET /kv/{key}/info`): Get detailed metadata about a specific key
- **Key Existence Check** (`GET /kv/{key}/exists`): Quickly check if a key exists without retrieving its value

#### Data Management
- **Batch Operations** (`POST /batch`): Set multiple key-value pairs in a single request using JSON array
- **Prefix-Based Deletion** (`DELETE /kv/prefix/{prefix}`): Delete all keys starting with a given prefix
- **Query Parameters for Listing**: Filter keys by prefix and limit results (`GET /kv/?prefix=foo&limit=10`)
- **Manual Database Compaction** (`POST /compact`): Trigger database file compaction on demand
- **Database Backup** (`POST /backup`): Create timestamped backup files of the entire database

#### Validation & Safety
- **Input Size Limits**: 
  - Maximum key size: 256 bytes
  - Maximum value size: 10 MB
- **Key Validation**: Empty keys are rejected
- **Value Validation**: Oversized values are rejected with clear error messages
- **Error Messages**: All endpoints now return descriptive error messages

#### Monitoring & Operations
- **Health Check Endpoint** (`GET /health`): Returns server status and current timestamp
- **Operations Counter**: Tracks total number of operations performed
- **Server Uptime Tracking**: Monitors how long the server has been running

#### API Improvements
- **HTTP Status Codes**: Proper use of 201 Created, 404 Not Found, 409 Conflict, etc.
- **JSON Responses**: Structured JSON responses for statistics, lists, and batch operations
- **Consistent Error Handling**: All errors return descriptive messages

### Changed
- **Updated Data Structure**: Internal storage now uses `KeyMetadata` struct instead of plain strings
- **Improved GET Operation**: Now increments access counter when retrieving values
- **Enhanced PUT Operation**: Returns 409 Conflict if key already exists instead of 400 Bad Request
- **Regex Search Results**: Now returns JSON array instead of newline-separated text
- **Version Bumped**: From 0.1.0 to 0.2.0

### Technical Improvements
- **Serialization Support**: Added serde and serde_json dependencies for JSON handling
- **Type Safety**: Strong typing for all data structures with Serialize/Deserialize traits
- **Better Concurrency**: Improved mutex handling to prevent deadlocks
- **Memory Efficiency**: Operations counter uses separate mutex to reduce contention

### Dependencies Added
- `serde = { version = "1.0", features = ["derive"] }`
- `serde_json = "1.0"`

## [0.1.0] - Initial Release

### Features
- Basic CRUD operations (GET, POST, PUT, DELETE)
- File-based persistence (kvstore.db)
- Thread-safe concurrent access
- Regex-based key search
- HTTP API with actix-web
- Compression middleware
- Request logging
