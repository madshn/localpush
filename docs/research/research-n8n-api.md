# n8n REST API Research for LocalPush Integration

**Date:** 2026-02-06
**Purpose:** Understand n8n API integration requirements for LocalPush webhook discovery

---

## 1. List Active Workflows

### Endpoint
```
GET https://n8n.rightaim.io/api/v1/workflows
```

### Authentication Header
```
X-N8N-API-KEY: <your-api-key>
```

The API key is obtained from n8n UI: Settings > n8n API

### Query Parameters
- `limit` (number, 1-100, default: 100) - Number of workflows to return
- `cursor` (string) - Pagination cursor from previous response
- `active` (boolean) - Filter by active/inactive status
- `tags` (array) - Filter by exact tag matches (AND logic)
- `projectId` (string) - Filter by project ID (enterprise feature)
- `excludePinnedData` (boolean, default: true) - Exclude pinned data

### Response Format
```json
{
  "workflows": [
    {
      "id": "tGOYYD5T0ePpeRIA",
      "name": "Analytics Events",
      "active": true,
      "isArchived": false,
      "createdAt": "2026-01-15T22:23:56.495Z",
      "updatedAt": "2026-01-21T12:53:05.305Z",
      "tags": [
        {
          "id": "nCZLDcj1WY9De5ZO",
          "name": "PLY",
          "createdAt": "2026-01-21T12:52:51.125Z",
          "updatedAt": "2026-01-21T12:52:51.125Z"
        }
      ],
      "nodeCount": 3
    }
  ],
  "returned": 5,
  "nextCursor": "eyJsaW1pdCI6NSwib2Zmc2V0Ijo1fQ==",
  "hasMore": true
}
```

### Pagination
- Check `hasMore` field to determine if more results exist
- Use `nextCursor` value in subsequent requests
- Iterate until `hasMore` is false for complete list
- `returned` is count of current page only, NOT total system count

### Example Request
```bash
curl -X GET "https://n8n.rightaim.io/api/v1/workflows?active=true&limit=100" \
  -H "X-N8N-API-KEY: your_api_key_here"
```

### Rate Limits
No explicit rate limits documented. API typically responds in 50-200ms for list operations.

---

## 2. Identify Webhook Trigger Nodes

### Limitation
**The list workflows endpoint does NOT return node details.** It only returns metadata:
- id, name, active, isArchived, createdAt, updatedAt, tags, nodeCount

To identify webhook triggers, you MUST:
1. List all workflows
2. For each workflow, fetch full details using GET `/api/v1/workflows/{id}`

### Webhook Node Type
Webhook trigger nodes have:
- `type: "n8n-nodes-base.webhook"`
- Present in workflow `nodes` array

### Example Detection (from full workflow response)
```json
{
  "nodes": [
    {
      "id": "webhook",
      "name": "Analytics Webhook",
      "type": "n8n-nodes-base.webhook",
      "typeVersion": 2,
      "webhookId": "a8e3ea97-8104-4523-9e84-7edc73848aa9",
      "parameters": {
        "httpMethod": "POST",
        "path": "ply-analytics",
        "authentication": "none",
        "responseMode": "responseNode",
        "options": {}
      }
    }
  ]
}
```

---

## 3. Extract Webhook URL from Node Configuration

### Webhook URL Construction

**Production URL Format:**
```
https://{n8n-instance}/webhook/{path}
```

**Test URL Format:**
```
https://{n8n-instance}/webhook-test/{path}
```

### Key Fields in Webhook Node

From node `parameters` object:

| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `path` | string | URL path segment (required) | `"ply-analytics"` |
| `httpMethod` | string or array | HTTP method(s) accepted | `"POST"` or `["GET", "POST"]` |
| `authentication` | string | Auth type: `"none"`, `"basicAuth"`, `"headerAuth"`, `"jwtAuth"` | `"none"` |
| `responseMode` | string | Response timing: `"onReceived"`, `"lastNode"`, `"responseNode"`, `"streaming"` | `"responseNode"` |
| `webhookId` | string (UUID) | Unique webhook identifier (at node level, not parameters) | `"a8e3ea97-8104-4523-9e84-7edc73848aa9"` |
| `options` | object | Additional settings (IP whitelist, CORS, etc.) | `{}` |

### Authentication Configuration

Authentication is NOT stored in node parameters. Instead, it references credentials:

```json
{
  "parameters": {
    "authentication": "headerAuth"
  },
  "credentials": {
    "httpHeaderAuth": {
      "id": "credential-id-here",
      "name": "Credential Display Name"
    }
  }
}
```

**Auth Types:**
- `none` - No authentication (public webhook)
- `basicAuth` - Basic auth (username/password)
- `headerAuth` - Custom header authentication
- `jwtAuth` - JWT token authentication

**Important:** The actual auth values (keys, passwords, tokens) are stored in the credentials system and NOT exposed via the API for security reasons.

### Example Full Webhook Node
```json
{
  "parameters": {
    "httpMethod": "POST",
    "path": "feedback",
    "authentication": "none",
    "responseMode": "responseNode",
    "options": {
      "rawBody": true
    }
  },
  "id": "webhook-feedback",
  "name": "Webhook: Feedback",
  "type": "n8n-nodes-base.webhook",
  "typeVersion": 2,
  "position": [192, 304],
  "webhookId": "f858df5e-428f-4303-ac42-f8e515fe7a6f"
}
```

**Resulting URL:** `https://n8n.rightaim.io/webhook/feedback`

---

## 4. Workflow Metadata Available

From `GET /api/v1/workflows` (list):
- `id` - Workflow ID (required for fetching full details)
- `name` - Display name
- `active` - Is workflow activated (true/false)
- `isArchived` - Is workflow archived (true/false)
- `createdAt` - ISO timestamp
- `updatedAt` - ISO timestamp
- `tags` - Array of tag objects with id, name, createdAt, updatedAt
- `nodeCount` - Total number of nodes in workflow

From `GET /api/v1/workflows/{id}` (full workflow):
- All list fields PLUS:
- `nodes` - Complete node configuration array
- `connections` - Node connection graph
- `settings` - Workflow settings (execution order, error handling, etc.)
- `versionId` - Current version identifier
- `activeVersionId` - Active version identifier
- `versionCounter` - Number of versions
- `triggerCount` - Number of trigger nodes
- `shared` - Sharing/project information
- `staticData` - Workflow static data
- `pinData` - Pinned test data

### Workflow Settings Object
```json
{
  "executionOrder": "v1",
  "saveDataErrorExecution": "all",
  "saveDataSuccessExecution": "all",
  "saveManualExecutions": true,
  "callerPolicy": "workflowsFromSameOwner",
  "availableInMCP": false
}
```

---

## 5. Detecting Webhook Authentication from Config

### Detection Strategy

1. **Check `authentication` parameter:**
   - `"none"` = No authentication (public)
   - Anything else = Authentication required

2. **Check `credentials` object:**
   - If present, webhook has authentication configured
   - Credential type matches auth parameter:
     - `basicAuth` → `httpBasicAuth`
     - `headerAuth` → `httpHeaderAuth`
     - `jwtAuth` → `jwtAuth`

3. **Check `options.ipWhitelist`:**
   - If present and non-empty, IP-based filtering is active
   - Format: `"127.0.0.1,192.168.1.0/24"` (comma-separated)

4. **CORS Settings in `options`:**
   - `allowedOrigins` - Permitted cross-origin domains
   - Default is `"*"` (allow all)

### Example: Authenticated Webhook
```json
{
  "parameters": {
    "httpMethod": "POST",
    "path": "secure-endpoint",
    "authentication": "headerAuth",
    "options": {
      "ipWhitelist": "192.168.1.0/24"
    }
  },
  "credentials": {
    "httpHeaderAuth": {
      "id": "ZsRdsfk4MvwFLMWo",
      "name": "Custom API Key"
    }
  }
}
```

**Detection Result:**
- Authentication: YES (headerAuth)
- IP Whitelist: YES (192.168.1.0/24)
- Public Access: NO

### Example: Public Webhook
```json
{
  "parameters": {
    "httpMethod": "POST",
    "path": "public-analytics",
    "authentication": "none",
    "options": {}
  }
}
```

**Detection Result:**
- Authentication: NO
- IP Whitelist: NO
- Public Access: YES

---

## Integration Workflow for LocalPush

### Recommended Flow

```
1. GET /api/v1/workflows?active=true&limit=100
   └─> Extract workflow IDs

2. For each workflow ID:
   GET /api/v1/workflows/{id}
   └─> Check nodes array for type === "n8n-nodes-base.webhook"

3. For each webhook node found:
   a. Extract path from parameters.path
   b. Construct URL: https://n8n.rightaim.io/webhook/{path}
   c. Extract httpMethod (string or array)
   d. Detect authentication:
      - parameters.authentication !== "none" = authenticated
      - Check credentials object for credential details (ID only, not values)
   e. Extract metadata:
      - Workflow name, tags, active status
      - Node name (webhook display name)
      - webhookId (unique identifier)

4. Store LocalPush registration:
   - Workflow ID + Node ID + Path + URL
   - HTTP methods supported
   - Authentication indicator (public vs. protected)
   - Metadata for display/filtering
```

### Performance Considerations

- List endpoint: ~50-200ms (fast)
- Full workflow fetch: ~200-500ms per workflow (slower)
- For 100 active workflows: ~50 seconds total
- Recommend caching strategy with TTL or webhook-based invalidation

### Security Notes

- API key should be stored securely (environment variable)
- Actual webhook authentication credentials are NOT exposed via API
- LocalPush can only detect IF auth is configured, not the actual secrets
- IP whitelist values ARE exposed (useful for validation)

---

## Additional API Endpoints (Future Use)

### Get Single Workflow
```
GET /api/v1/workflows/{id}?mode=full
```

Modes:
- `full` - Complete workflow (default)
- `details` - Metadata + execution stats
- `structure` - Nodes/connections only
- `minimal` - id/name/active/tags only

### Activate/Deactivate Workflow
```
PATCH /api/v1/workflows/{id}
Body: { "active": true/false }
```

### Webhook Re-registration
After trigger changes, workflows must be deactivated then reactivated to re-register webhooks with n8n's webhook handler.

---

## Sources

- [n8n API Documentation](https://docs.n8n.io/api/)
- [n8n API Reference](https://docs.n8n.io/api/api-reference/)
- [n8n API Authentication](https://docs.n8n.io/api/authentication/)
- [Webhook Node Documentation](https://docs.n8n.io/integrations/builtin/core-nodes/n8n-nodes-base.webhook/)
- [Webhook Credentials](https://docs.n8n.io/integrations/builtin/credentials/webhook/)
- n8n MCP Tools (custom implementation)
