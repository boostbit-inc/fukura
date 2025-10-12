# FukuraHub Requirements Specification

World-class backend service for Fukura CLI, inspired by GitHub's architecture and user experience.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Authentication & Authorization](#authentication--authorization)
- [User Management](#user-management)
- [Organization Management](#organization-management)
- [Note Storage & Versioning](#note-storage--versioning)
- [Search & Discovery](#search--discovery)
- [Real-time Collaboration](#real-time-collaboration)
- [API Specification](#api-specification)
- [Security](#security)
- [Performance & Scalability](#performance--scalability)
- [Analytics & Insights](#analytics--insights)
- [CLI Integration](#cli-integration)
- [Missing CLI Features](#missing-cli-features)

---

## Overview

FukuraHub is a collaborative knowledge-sharing platform that enables teams to:
- **Store** error solutions and technical notes privately or in organizations
- **Search** across team knowledge bases with powerful full-text search
- **Sync** notes automatically from local repositories to the cloud
- **Collaborate** with team members in real-time
- **Discover** solutions from public notes across the community
- **Analyze** team learning patterns and knowledge gaps

### Core Principles

1. **Privacy First** - Users control visibility (private/org/public)
2. **Git-like Workflow** - Familiar push/pull/sync operations
3. **Offline Capable** - Full functionality without internet
4. **Team-Centric** - Organizations and permissions like GitHub
5. **Developer-Friendly** - API-first design, extensive CLI integration
6. **Fast & Scalable** - Sub-100ms latency, handles millions of notes

---

## Architecture

### Technology Stack

#### Backend
- **API Server**: Rust (Axum/Actix-Web) or Go (Gin/Echo)
  - High performance, type safety, memory efficiency
  - Native async/await for concurrent operations
  - Strong ecosystem for crypto and security

- **Database**:
  - **Primary**: PostgreSQL (with JSON/JSONB for metadata)
  - **Cache**: Redis (session data, rate limiting, real-time presence)
  - **Search**: Elasticsearch or Meilisearch (full-text search)
  - **Object Storage**: S3/MinIO (large attachments, pack files)

#### Infrastructure
- **Load Balancer**: Nginx or HAProxy
- **CDN**: Cloudflare or Fastly (static assets, API edge caching)
- **Message Queue**: RabbitMQ or NATS (async jobs, notifications)
- **Monitoring**: Prometheus + Grafana
- **Logging**: ELK Stack or Loki
- **Tracing**: Jaeger or OpenTelemetry

### System Design

```
┌─────────────┐
│  Fukura CLI │
│  (Local)    │
└──────┬──────┘
       │ HTTPS/WebSocket
       ↓
┌─────────────────────────────────────────┐
│         Load Balancer (Nginx)           │
│  - SSL Termination                      │
│  - Rate Limiting                        │
│  - DDoS Protection                      │
└──────┬────────────────────┬─────────────┘
       │                    │
       ↓                    ↓
┌─────────────┐      ┌─────────────┐
│  API Server │      │  API Server │
│  (Rust/Go)  │      │  (Rust/Go)  │
│  - REST API │      │  - REST API │
│  - WebSocket│      │  - WebSocket│
└──────┬──────┘      └──────┬──────┘
       │                    │
       └──────────┬──────────┘
                  │
      ┌───────────┼──────────────┐
      │           │              │
      ↓           ↓              ↓
┌──────────┐ ┌──────────┐  ┌──────────┐
│PostgreSQL│ │  Redis   │  │  Search  │
│          │ │  Cache   │  │  Engine  │
│ - Users  │ │ - Sessions│  │ - Index  │
│ - Notes  │ │ - Rate   │  │ - Query  │
│ - Orgs   │ │   Limit  │  │          │
└──────────┘ └──────────┘  └──────────┘
      │
      ↓
┌──────────┐
│   S3     │
│ Storage  │
│ - Packs  │
│ - Assets │
└──────────┘
```

### Microservices (Future)

- **Note Service**: CRUD operations for notes
- **Search Service**: Indexing and querying
- **Auth Service**: Authentication and authorization
- **Org Service**: Organization and team management
- **Sync Service**: Real-time synchronization
- **Analytics Service**: Usage metrics and insights
- **Notification Service**: Webhooks and alerts

---

## Authentication & Authorization

### Authentication Methods

#### 1. Personal Access Tokens (Primary for CLI)

Similar to GitHub's personal access tokens:

```bash
# User generates token on web dashboard
# Token format: fkh_<base62_random_32_chars>
# Example: fkh_7XkqP9vRm2TnZbC4sY8wLdF3gH6jK

# CLI usage
fuku config remote --set https://hub.fukura.dev
export FUKURA_TOKEN=fkh_7XkqP9vRm2TnZbC4sY8wLdF3gH6jK
fuku push @latest
```

**Token Scopes** (fine-grained permissions):
- `note:read` - Read private notes
- `note:write` - Create/update notes
- `note:delete` - Delete notes
- `org:read` - Read organization data
- `org:write` - Manage organization settings
- `user:read` - Read user profile
- `user:write` - Update user profile
- `admin` - Full access

**Token Management**:
- Create with expiration (7 days, 30 days, 90 days, no expiration)
- Revoke tokens instantly
- View last used timestamp and IP
- Audit log of all token usage

#### 2. OAuth 2.0 (Web Dashboard)

Support major providers:
- GitHub OAuth
- Google OAuth
- Azure AD (for enterprises)
- SAML 2.0 (for enterprises)

#### 3. API Keys (Service Accounts)

For automated systems and CI/CD:
- API key format: `fkh_svc_<base62_random_32_chars>`
- Tied to service accounts, not users
- Separate rate limits
- Cannot be used for interactive sessions

### Authorization Model

#### Role-Based Access Control (RBAC)

**Organization Roles**:
- **Owner** - Full control, billing, delete org
- **Admin** - Manage members, teams, settings
- **Member** - Create/view org notes
- **Guest** - Read-only access to public notes

**Team Roles** (within organization):
- **Maintainer** - Manage team members, note permissions
- **Member** - Contribute to team notes
- **Read-Only** - View team notes only

#### Permission Levels for Notes

| Privacy Level | Who Can View | Who Can Edit |
|---------------|--------------|--------------|
| `private` | Owner only | Owner only |
| `org` | Org members | Owner + assigned editors |
| `public` | Anyone | Owner + collaborators |

### Session Management

- **JWT tokens** for web sessions
- **Redis** for session storage
- **Refresh tokens** with 30-day expiration
- **Automatic logout** after 90 days of inactivity
- **Device tracking** - List active sessions per device

---

## User Management

### User Schema

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(39) UNIQUE NOT NULL,  -- GitHub username length
    email VARCHAR(255) UNIQUE NOT NULL,
    email_verified BOOLEAN DEFAULT FALSE,
    display_name VARCHAR(255),
    avatar_url VARCHAR(512),
    bio TEXT,
    website VARCHAR(255),
    location VARCHAR(100),
    
    -- Authentication
    password_hash VARCHAR(255),  -- bcrypt
    oauth_provider VARCHAR(50),
    oauth_id VARCHAR(255),
    
    -- Settings
    default_privacy VARCHAR(20) DEFAULT 'private',
    auto_sync BOOLEAN DEFAULT FALSE,
    redaction_enabled BOOLEAN DEFAULT TRUE,
    
    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ,
    
    -- Status
    is_active BOOLEAN DEFAULT TRUE,
    is_verified BOOLEAN DEFAULT FALSE,
    is_staff BOOLEAN DEFAULT FALSE,
    
    -- Stats (denormalized for performance)
    note_count INTEGER DEFAULT 0,
    follower_count INTEGER DEFAULT 0,
    following_count INTEGER DEFAULT 0,
    
    CONSTRAINT username_format CHECK (username ~* '^[a-z0-9][a-z0-9-]{0,38}$'),
    CONSTRAINT email_format CHECK (email ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}$')
);

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_created_at ON users(created_at);
```

### User Profile API

```bash
# Get user profile
GET /api/v1/users/:username

# Update current user profile
PATCH /api/v1/user
{
  "display_name": "John Doe",
  "bio": "Full-stack developer",
  "website": "https://example.com"
}

# Get user's public notes
GET /api/v1/users/:username/notes?page=1&limit=20

# Get user's stats
GET /api/v1/users/:username/stats
```

### User Settings

```bash
# Get all settings
GET /api/v1/user/settings

# Update settings
PATCH /api/v1/user/settings
{
  "default_privacy": "org",
  "auto_sync": true,
  "email_notifications": {
    "mentions": true,
    "org_updates": false
  }
}
```

### Social Features

#### Following/Followers

```bash
# Follow user
POST /api/v1/users/:username/follow

# Unfollow user
DELETE /api/v1/users/:username/follow

# Get followers
GET /api/v1/users/:username/followers

# Get following
GET /api/v1/users/:username/following
```

#### Activity Feed

```bash
# Get user's activity
GET /api/v1/users/:username/activity?page=1&limit=20

Response:
{
  "activities": [
    {
      "id": "act_123",
      "type": "note_created",
      "user": "johndoe",
      "note": {
        "id": "a3f8e9b2",
        "title": "Fixed CORS issue"
      },
      "timestamp": "2024-10-10T15:30:00Z"
    },
    {
      "type": "note_liked",
      "user": "janedoe",
      "note_id": "b4c7d8e1",
      "timestamp": "2024-10-10T14:20:00Z"
    }
  ],
  "page": 1,
  "total": 42
}
```

---

## Organization Management

### Organization Schema

```sql
CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug VARCHAR(39) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    avatar_url VARCHAR(512),
    website VARCHAR(255),
    
    -- Settings
    default_privacy VARCHAR(20) DEFAULT 'org',
    require_2fa BOOLEAN DEFAULT FALSE,
    allow_public_notes BOOLEAN DEFAULT TRUE,
    
    -- Billing
    plan VARCHAR(50) DEFAULT 'free',  -- free, team, enterprise
    billing_email VARCHAR(255),
    
    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- Stats
    member_count INTEGER DEFAULT 0,
    team_count INTEGER DEFAULT 0,
    note_count INTEGER DEFAULT 0
);

CREATE TABLE organization_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL,  -- owner, admin, member, guest
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    invited_by UUID REFERENCES users(id),
    
    UNIQUE(organization_id, user_id)
);

CREATE TABLE teams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    slug VARCHAR(100) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    parent_team_id UUID REFERENCES teams(id) ON DELETE SET NULL,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(organization_id, slug)
);

CREATE TABLE team_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id UUID REFERENCES teams(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL,  -- maintainer, member, read-only
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(team_id, user_id)
);
```

### Organization API

```bash
# Create organization
POST /api/v1/organizations
{
  "slug": "acme-corp",
  "name": "Acme Corporation",
  "description": "Best widgets in the world"
}

# Get organization
GET /api/v1/organizations/:slug

# Update organization
PATCH /api/v1/organizations/:slug
{
  "name": "Acme Corp",
  "website": "https://acme.com"
}

# Delete organization (owner only)
DELETE /api/v1/organizations/:slug
```

### Member Management

```bash
# List members
GET /api/v1/organizations/:slug/members?page=1&limit=50

# Invite member
POST /api/v1/organizations/:slug/members
{
  "email": "user@example.com",
  "role": "member"
}

# Update member role
PATCH /api/v1/organizations/:slug/members/:username
{
  "role": "admin"
}

# Remove member
DELETE /api/v1/organizations/:slug/members/:username
```

### Team Management

```bash
# Create team
POST /api/v1/organizations/:slug/teams
{
  "slug": "backend",
  "name": "Backend Team",
  "description": "Backend engineers"
}

# List teams
GET /api/v1/organizations/:slug/teams

# Add team member
POST /api/v1/teams/:team_id/members
{
  "username": "johndoe",
  "role": "member"
}

# Get team notes
GET /api/v1/teams/:team_id/notes
```

### Organization Settings

```bash
# Get settings
GET /api/v1/organizations/:slug/settings

# Update settings
PATCH /api/v1/organizations/:slug/settings
{
  "default_privacy": "org",
  "require_2fa": true,
  "allowed_domains": ["acme.com"]
}

# Audit log
GET /api/v1/organizations/:slug/audit-log?page=1&limit=100
```

---

## Note Storage & Versioning

### Note Schema

```sql
CREATE TABLE notes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    object_id VARCHAR(64) UNIQUE NOT NULL,  -- SHA-256 hash
    
    -- Ownership
    owner_id UUID REFERENCES users(id) ON DELETE CASCADE,
    organization_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
    team_id UUID REFERENCES teams(id) ON DELETE SET NULL,
    
    -- Content
    title VARCHAR(500) NOT NULL,
    body TEXT NOT NULL,
    tags TEXT[] DEFAULT '{}',
    links TEXT[] DEFAULT '{}',
    metadata JSONB DEFAULT '{}',
    
    -- Author (can differ from owner for imports)
    author_name VARCHAR(255),
    author_email VARCHAR(255),
    
    -- Privacy
    privacy VARCHAR(20) NOT NULL,  -- private, org, public
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    synced_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- Stats
    view_count INTEGER DEFAULT 0,
    like_count INTEGER DEFAULT 0,
    fork_count INTEGER DEFAULT 0,
    
    -- Version tracking
    version INTEGER DEFAULT 1,
    parent_id UUID REFERENCES notes(id) ON DELETE SET NULL,
    
    -- Search
    search_vector tsvector GENERATED ALWAYS AS (
        setweight(to_tsvector('english', COALESCE(title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(body, '')), 'B') ||
        setweight(to_tsvector('english', array_to_string(tags, ' ')), 'C')
    ) STORED
);

CREATE INDEX idx_notes_object_id ON notes(object_id);
CREATE INDEX idx_notes_owner ON notes(owner_id);
CREATE INDEX idx_notes_org ON notes(organization_id);
CREATE INDEX idx_notes_team ON notes(team_id);
CREATE INDEX idx_notes_privacy ON notes(privacy);
CREATE INDEX idx_notes_created_at ON notes(created_at DESC);
CREATE INDEX idx_notes_search ON notes USING GIN(search_vector);
CREATE INDEX idx_notes_tags ON notes USING GIN(tags);

-- Note versions (for history)
CREATE TABLE note_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    note_id UUID REFERENCES notes(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    object_id VARCHAR(64) NOT NULL,
    content JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    
    UNIQUE(note_id, version)
);

-- Note likes
CREATE TABLE note_likes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    note_id UUID REFERENCES notes(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(note_id, user_id)
);

-- Note views (for analytics)
CREATE TABLE note_views (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    note_id UUID REFERENCES notes(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    ip_address INET,
    user_agent TEXT,
    viewed_at TIMESTAMPTZ DEFAULT NOW()
);

-- Note comments (future feature)
CREATE TABLE note_comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    note_id UUID REFERENCES notes(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    parent_comment_id UUID REFERENCES note_comments(id) ON DELETE CASCADE,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);
```

### Note API

```bash
# Create note (push from CLI)
POST /api/v1/notes
Authorization: Bearer fkh_...
{
  "object_id": "a3f8e9b2c4d5e6f7...",
  "title": "Fixed CORS issue",
  "body": "Added Access-Control-Allow-Origin...",
  "tags": ["cors", "nginx"],
  "links": ["https://example.com"],
  "metadata": {
    "severity": "high"
  },
  "privacy": "org",
  "created_at": "2024-10-10T15:30:00Z",
  "updated_at": "2024-10-10T15:30:00Z"
}

Response:
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "object_id": "a3f8e9b2c4d5e6f7...",
  "url": "https://hub.fukura.dev/notes/a3f8e9b2",
  "web_url": "https://hub.fukura.dev/@johndoe/notes/a3f8e9b2"
}

# Get note (pull from CLI)
GET /api/v1/notes/:object_id

# Update note
PATCH /api/v1/notes/:object_id
{
  "body": "Updated content...",
  "tags": ["cors", "nginx", "production"]
}

# Delete note
DELETE /api/v1/notes/:object_id

# Get note versions (history)
GET /api/v1/notes/:object_id/versions

# Get specific version
GET /api/v1/notes/:object_id/versions/:version

# Fork note (create editable copy)
POST /api/v1/notes/:object_id/fork
{
  "privacy": "private"
}

# Like note
POST /api/v1/notes/:object_id/like

# Unlike note
DELETE /api/v1/notes/:object_id/like
```

### Batch Operations

```bash
# Batch push (sync --all from CLI)
POST /api/v1/notes/batch
Authorization: Bearer fkh_...
{
  "notes": [
    { "object_id": "a3f8...", "title": "Note 1", ... },
    { "object_id": "b4c7...", "title": "Note 2", ... }
  ]
}

Response:
{
  "success": [
    { "object_id": "a3f8...", "id": "550e8400..." },
    { "object_id": "b4c7...", "id": "660f9511..." }
  ],
  "failed": []
}

# Batch get (optimization)
POST /api/v1/notes/batch/get
{
  "object_ids": ["a3f8...", "b4c7...", "c5d9..."]
}
```

### Pack File Support

For efficient bulk sync (like Git):

```bash
# Upload pack file
POST /api/v1/notes/pack
Content-Type: application/octet-stream
Content-Length: 1048576

<binary pack file data>

Response:
{
  "pack_id": "pack-a3f8e9b2",
  "objects": 42,
  "size": 1048576
}

# Download pack file
GET /api/v1/notes/pack/:pack_id
```

---

## Search & Discovery

### Search API

```bash
# Full-text search
GET /api/v1/search/notes?q=redis+timeout&limit=20&page=1&sort=relevance

Query Parameters:
- q: Search query (supports operators: AND, OR, NOT, "exact phrase")
- limit: Results per page (max 100)
- page: Page number
- sort: relevance | updated | likes | created
- privacy: Filter by privacy level
- tags: Filter by tags (comma-separated)
- author: Filter by author username
- org: Filter by organization slug
- date_from: ISO 8601 date (created after)
- date_to: ISO 8601 date (created before)

Response:
{
  "results": [
    {
      "id": "a3f8e9b2",
      "title": "Fixed Redis timeout issue",
      "snippet": "...increased timeout to <mark>30 seconds</mark>...",
      "tags": ["redis", "timeout"],
      "author": "johndoe",
      "privacy": "public",
      "likes": 15,
      "created_at": "2024-10-10T15:30:00Z",
      "score": 0.95
    }
  ],
  "total": 127,
  "page": 1,
  "pages": 7,
  "took_ms": 12
}
```

### Advanced Search Operators

```
# Exact phrase
"redis connection error"

# Boolean operators
redis AND timeout
redis OR connection
redis NOT cluster

# Field-specific search
title:redis
tag:database
author:johndoe
org:acme-corp

# Wildcards
redi*
conn?ction

# Date ranges
created:>2024-01-01
updated:<2024-10-01

# Combine operators
title:"redis error" AND tag:production AND author:johndoe created:>2024-09-01
```

### Trending Notes

```bash
# Get trending notes
GET /api/v1/trending/notes?timeframe=week&limit=20

Response:
{
  "notes": [
    {
      "id": "a3f8e9b2",
      "title": "Kubernetes pod crashloop debugging",
      "tags": ["kubernetes", "debugging"],
      "likes": 127,
      "views": 1543,
      "trending_score": 0.98
    }
  ]
}
```

### Tag Cloud

```bash
# Get popular tags
GET /api/v1/tags?limit=100

Response:
{
  "tags": [
    { "name": "kubernetes", "count": 1234, "trending": true },
    { "name": "redis", "count": 891 },
    { "name": "docker", "count": 765 }
  ]
}

# Get notes by tag
GET /api/v1/tags/:tag/notes?page=1&limit=20
```

### Suggestions & Recommendations

```bash
# Get similar notes
GET /api/v1/notes/:id/similar?limit=10

# Get personalized recommendations
GET /api/v1/recommendations?limit=20

# Auto-complete search
GET /api/v1/search/suggestions?q=kuber&limit=10

Response:
{
  "suggestions": [
    "kubernetes",
    "kubernetes pod",
    "kubernetes deployment"
  ]
}
```

---

## Real-time Collaboration

### WebSocket Connection

```javascript
// Establish WebSocket connection
const ws = new WebSocket('wss://hub.fukura.dev/api/v1/ws');
ws.send(JSON.stringify({
  type: 'auth',
  token: 'fkh_...'
}));

// Subscribe to note updates
ws.send(JSON.stringify({
  type: 'subscribe',
  resource: 'note',
  id: 'a3f8e9b2'
}));

// Receive real-time updates
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  // { type: 'note_updated', id: 'a3f8e9b2', ... }
};
```

### Event Types

```javascript
// Note events
{ type: 'note_created', note: {...} }
{ type: 'note_updated', note: {...} }
{ type: 'note_deleted', id: '...' }
{ type: 'note_liked', note_id: '...', user: '...' }

// Comment events
{ type: 'comment_created', note_id: '...', comment: {...} }
{ type: 'comment_updated', comment: {...} }

// Organization events
{ type: 'member_joined', org: '...', user: '...' }
{ type: 'team_created', org: '...', team: {...} }

// Presence events
{ type: 'user_online', user: '...' }
{ type: 'user_offline', user: '...' }
{ type: 'user_viewing', note_id: '...', user: '...' }
```

### Presence System

```bash
# Update presence
POST /api/v1/presence
{
  "status": "online",
  "current_note": "a3f8e9b2"
}

# Get who's viewing a note
GET /api/v1/notes/:id/viewers

Response:
{
  "viewers": [
    {
      "username": "johndoe",
      "avatar_url": "...",
      "viewing_since": "2024-10-10T15:30:00Z"
    }
  ]
}
```

### Collaborative Editing (Future)

- Operational Transform (OT) or CRDT for conflict-free editing
- Real-time cursor positions
- Live markdown preview sync
- Collaborative annotation

---

## API Specification

### REST API Design Principles

1. **Versioning**: `/api/v1/...`
2. **Resource-oriented**: `/notes`, `/users`, `/organizations`
3. **HTTP verbs**: GET, POST, PATCH, DELETE
4. **JSON responses**: Always `application/json`
5. **Pagination**: `page` and `limit` query parameters
6. **Filtering**: Query parameters for filtering
7. **Rate limiting**: Per-user and per-IP limits
8. **Error handling**: Consistent error format

### Authentication

All requests require authentication header:

```
Authorization: Bearer fkh_7XkqP9vRm2TnZbC4sY8wLdF3gH6jK
```

### Response Format

#### Success Response

```json
{
  "data": { ... },
  "meta": {
    "page": 1,
    "limit": 20,
    "total": 127
  }
}
```

#### Error Response

```json
{
  "error": {
    "code": "invalid_request",
    "message": "Title is required",
    "details": {
      "field": "title",
      "constraint": "required"
    }
  }
}
```

### Error Codes

| HTTP Status | Error Code | Description |
|-------------|------------|-------------|
| 400 | `invalid_request` | Malformed request |
| 401 | `unauthorized` | Authentication required |
| 403 | `forbidden` | Permission denied |
| 404 | `not_found` | Resource not found |
| 409 | `conflict` | Resource already exists |
| 422 | `validation_error` | Validation failed |
| 429 | `rate_limit_exceeded` | Too many requests |
| 500 | `internal_error` | Server error |
| 503 | `service_unavailable` | Maintenance mode |

### Rate Limiting

```
X-RateLimit-Limit: 5000
X-RateLimit-Remaining: 4999
X-RateLimit-Reset: 1696963200
```

**Limits**:
- **Free tier**: 60 requests/minute, 5,000 requests/hour
- **Team tier**: 200 requests/minute, 20,000 requests/hour
- **Enterprise tier**: Custom limits

### Pagination

```bash
GET /api/v1/notes?page=2&limit=50

Response Headers:
Link: <https://hub.fukura.dev/api/v1/notes?page=3&limit=50>; rel="next",
      <https://hub.fukura.dev/api/v1/notes?page=1&limit=50>; rel="prev",
      <https://hub.fukura.dev/api/v1/notes?page=10&limit=50>; rel="last"
```

### Webhooks

Organizations can configure webhooks for events:

```bash
# Create webhook
POST /api/v1/organizations/:slug/webhooks
{
  "url": "https://example.com/webhook",
  "events": ["note_created", "note_updated"],
  "secret": "webhook_secret_key"
}

# Webhook payload
POST https://example.com/webhook
X-Fukura-Event: note_created
X-Fukura-Signature: sha256=...
{
  "event": "note_created",
  "organization": "acme-corp",
  "note": {
    "id": "a3f8e9b2",
    "title": "New note",
    ...
  },
  "timestamp": "2024-10-10T15:30:00Z"
}
```

---

## Security

### Encryption

- **TLS 1.3** for all connections
- **At-rest encryption** for database and storage (AES-256)
- **Password hashing** with bcrypt (cost factor 12)
- **Token encryption** with AES-256-GCM

### Input Validation

- Sanitize all user input
- Validate data types and formats
- Prevent SQL injection (use parameterized queries)
- Prevent XSS (escape HTML output)
- Prevent CSRF (use tokens)

### Content Security Policy

```
Content-Security-Policy: default-src 'self'; 
                          script-src 'self' 'unsafe-inline'; 
                          style-src 'self' 'unsafe-inline'; 
                          img-src 'self' data: https:;
```

### Audit Logging

Track all sensitive operations:
- Authentication attempts (success/failure)
- Note create/update/delete
- Permission changes
- Organization settings changes
- API key generation/revocation

### Compliance

- **GDPR**: Right to erasure, data portability, consent management
- **SOC 2**: Security controls and audit reports
- **HIPAA** (optional): For healthcare organizations

---

## Performance & Scalability

### Performance Targets

- **API latency**: < 100ms p50, < 300ms p99
- **Search latency**: < 200ms p50, < 500ms p99
- **WebSocket latency**: < 50ms
- **Throughput**: 10,000 requests/second per server

### Caching Strategy

1. **Redis Cache**:
   - User sessions (30 min TTL)
   - Popular notes (10 min TTL)
   - Search results (5 min TTL)
   - User profiles (15 min TTL)

2. **CDN Cache**:
   - Static assets (1 year)
   - API responses (1 min)
   - Public note pages (5 min)

3. **Database Query Cache**:
   - Frequent queries cached in memory
   - Invalidate on write operations

### Database Optimization

- **Indexing**: Proper indices on all query columns
- **Connection pooling**: 20-50 connections per server
- **Read replicas**: Route reads to replicas
- **Partitioning**: Partition notes table by created_at (monthly)
- **Archiving**: Move old notes to cold storage

### Horizontal Scaling

- **Load balancer**: Distribute traffic across API servers
- **Database replication**: Master-slave for read scaling
- **Sharding** (future): Shard by organization_id
- **Microservices** (future): Split services by domain

---

## Analytics & Insights

### User Analytics

```bash
# Get user stats
GET /api/v1/users/:username/stats

Response:
{
  "notes": {
    "total": 127,
    "private": 89,
    "org": 32,
    "public": 6
  },
  "activity": {
    "notes_this_week": 5,
    "notes_this_month": 18
  },
  "engagement": {
    "total_likes": 245,
    "total_views": 3421,
    "followers": 42
  },
  "top_tags": [
    { "tag": "kubernetes", "count": 23 },
    { "tag": "redis", "count": 15 }
  ]
}
```

### Organization Analytics

```bash
# Get organization insights
GET /api/v1/organizations/:slug/insights

Response:
{
  "overview": {
    "total_notes": 1543,
    "total_members": 42,
    "total_teams": 8
  },
  "activity": {
    "notes_created_this_week": 23,
    "active_members": 35
  },
  "top_contributors": [
    { "username": "johndoe", "notes": 89 },
    { "username": "janedoe", "notes": 67 }
  ],
  "top_tags": [
    { "tag": "kubernetes", "count": 234 },
    { "tag": "docker", "count": 187 }
  ],
  "knowledge_gaps": [
    { "topic": "kafka", "queries": 45, "notes": 2 }
  ]
}
```

### Search Analytics

Track search queries to identify:
- Popular topics
- Knowledge gaps (searches with few results)
- Trending technologies
- Seasonal patterns

---

## CLI Integration

### SDK for Rust (Client Library)

```rust
use fukurahub_client::{Client, Note, Privacy};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new("https://hub.fukura.dev", "fkh_...")?;
    
    // Push note
    let note = Note {
        object_id: "a3f8e9b2...",
        title: "Fixed CORS issue",
        body: "Content...",
        tags: vec!["cors", "nginx"],
        privacy: Privacy::Org,
        ..Default::default()
    };
    
    let response = client.notes().push(&note).await?;
    println!("Pushed: {}", response.id);
    
    // Pull note
    let note = client.notes().get("a3f8e9b2").await?;
    println!("Title: {}", note.title);
    
    // Search
    let results = client.search()
        .query("redis timeout")
        .limit(20)
        .execute()
        .await?;
    
    for hit in results {
        println!("{}: {}", hit.title, hit.score);
    }
    
    Ok(())
}
```

### CLI Commands Integration

All CLI commands should seamlessly integrate with FukuraHub:

```bash
# Configuration
fuku config remote --set https://hub.fukura.dev
fuku config remote --token fkh_...

# Or use environment variable
export FUKURA_TOKEN=fkh_7XkqP9vRm2TnZbC4sY8wLdF3gH6jK

# Push/pull work automatically
fuku push @latest
fuku pull hub-id-12345

# Sync all private notes
fuku sync --all

# Search hub (future)
fuku search "kubernetes" --remote

# Clone organization notes (future)
fuku clone org/acme-corp
```

---

## Missing CLI Features

### Features to Add to CLI

#### 1. Remote Search

```bash
# Search remote hub instead of local
fuku search "kubernetes" --remote

# Search specific organization
fuku search "redis" --org acme-corp

# Combine local and remote search
fuku search "docker" --all-sources
```

**Implementation**: Add `--remote` flag to search command, call FukuraHub API.

#### 2. Note Cloning

```bash
# Clone public note from hub
fuku clone https://hub.fukura.dev/@johndoe/notes/a3f8e9b2

# Clone entire organization's notes
fuku clone org/acme-corp

# Clone with specific privacy
fuku clone hub-id-12345 --privacy private
```

**Implementation**: New `clone` command that downloads notes from hub.

#### 3. Collaboration

```bash
# Share note with user
fuku share a3f8e9b2 --with johndoe

# Share note with team
fuku share a3f8e9b2 --team backend

# List collaborators
fuku collaborators a3f8e9b2

# Revoke access
fuku unshare a3f8e9b2 --from johndoe
```

**Implementation**: New `share`/`unshare`/`collaborators` commands.

#### 4. Comments

```bash
# Add comment to note
fuku comment a3f8e9b2 "This worked for me too!"

# List comments
fuku comments a3f8e9b2

# Reply to comment
fuku reply comment-id-123 "Thanks for confirming"
```

**Implementation**: New `comment`/`comments`/`reply` commands.

#### 5. Likes

```bash
# Like note
fuku like a3f8e9b2

# Unlike note
fuku unlike a3f8e9b2

# Show liked notes
fuku liked
```

**Implementation**: New `like`/`unlike`/`liked` commands.

#### 6. Following

```bash
# Follow user
fuku follow johndoe

# Unfollow user
fuku unfollow johndoe

# List following
fuku following

# List followers
fuku followers
```

**Implementation**: New `follow`/`unfollow`/`following`/`followers` commands.

#### 7. Organization Management

```bash
# Create organization
fuku org create acme-corp --name "Acme Corporation"

# List organizations
fuku orgs

# Switch organization context
fuku org use acme-corp

# Invite member
fuku org invite user@example.com --role member

# List members
fuku org members
```

**Implementation**: New `org` subcommand with multiple sub-subcommands.

#### 8. Team Management

```bash
# Create team
fuku team create backend --org acme-corp

# Add team member
fuku team add johndoe --team backend

# List teams
fuku teams --org acme-corp
```

**Implementation**: New `team` subcommand.

#### 9. Trending & Discovery

```bash
# Show trending notes
fuku trending --timeframe week

# Show popular tags
fuku tags --limit 50

# Recommendations
fuku discover
```

**Implementation**: New `trending`/`tags`/`discover` commands.

#### 10. Statistics

```bash
# Personal stats
fuku stats

# Organization stats
fuku stats --org acme-corp

# User stats
fuku stats --user johndoe
```

**Implementation**: New `stats` command.

#### 11. Export/Import

```bash
# Export all notes to JSON
fuku export --output backup.json

# Export to Markdown
fuku export --format markdown --output notes/

# Import from JSON
fuku import backup.json

# Import from Markdown files
fuku import --format markdown --input notes/
```

**Implementation**: New `export`/`import` commands.

#### 12. Watch Mode

```bash
# Watch for note updates in real-time
fuku watch a3f8e9b2

# Watch organization activity
fuku watch --org acme-corp

# Watch specific tag
fuku watch --tag kubernetes
```

**Implementation**: New `watch` command with WebSocket integration.

---

## Implementation Roadmap

### Phase 1: MVP (3 months)

- [ ] User authentication (OAuth, tokens)
- [ ] Basic CRUD for notes
- [ ] Push/pull/sync API
- [ ] Search API (basic)
- [ ] CLI integration
- [ ] Web dashboard (basic)

### Phase 2: Organizations (2 months)

- [ ] Organization management
- [ ] Member roles and permissions
- [ ] Team management
- [ ] Organization notes

### Phase 3: Collaboration (2 months)

- [ ] Comments
- [ ] Likes
- [ ] Following/followers
- [ ] Activity feeds
- [ ] Real-time updates (WebSocket)

### Phase 4: Advanced Features (3 months)

- [ ] Advanced search (filters, operators)
- [ ] Trending and recommendations
- [ ] Analytics dashboard
- [ ] Webhooks
- [ ] API rate limiting

### Phase 5: Enterprise (3 months)

- [ ] SAML/SSO integration
- [ ] Audit logging
- [ ] Compliance features
- [ ] Self-hosted option
- [ ] Advanced security

### Phase 6: Scale (ongoing)

- [ ] Performance optimization
- [ ] Horizontal scaling
- [ ] Microservices architecture
- [ ] Global CDN
- [ ] 99.99% uptime SLA

---

## Conclusion

FukuraHub will be a world-class knowledge-sharing platform that:
- Seamlessly integrates with Fukura CLI
- Provides GitHub-level collaboration features
- Scales to millions of users and notes
- Maintains sub-100ms latency
- Protects user privacy and security
- Empowers teams to build knowledge bases

The architecture is designed for:
- **Performance**: Fast response times, efficient caching
- **Scalability**: Horizontal scaling, microservices-ready
- **Security**: Encryption, audit logging, compliance
- **Reliability**: High availability, disaster recovery
- **Developer Experience**: Clean APIs, comprehensive docs

This specification provides a complete blueprint for building FukuraHub at the highest professional standard, comparable to GitHub, GitLab, and other world-class platforms.

