# ClovaLink Extension Developer Guide

This guide explains how to develop and publish extensions for ClovaLink.

## Extension Types

ClovaLink supports three types of extensions:

1. **UI Extensions** - Inject buttons, components, and sidebar items into the ClovaLink interface
2. **File Processor Extensions** - Triggered when users upload files
3. **Automation Extensions** - Run on a schedule or triggered manually

## Creating an Extension

### Step 1: Create a Manifest

Create a `manifest.json` file and host it at a publicly accessible URL:

```json
{
  "name": "My Extension",
  "slug": "my-extension",
  "version": "1.0.0",
  "type": "ui",
  "description": "A sample UI extension",
  "permissions": ["read:files"],
  "ui": {
    "load_mode": "iframe",
    "sidebar": [
      {
        "id": "my-sidebar-panel",
        "name": "My Panel",
        "icon": "https://example.com/icon.svg",
        "entrypoint": "https://example.com/extension/sidebar.html",
        "order": 100
      }
    ],
    "buttons": [
      {
        "id": "my-button",
        "name": "My Action",
        "location": "file_actions",
        "entrypoint": "https://example.com/extension/action.html"
      }
    ]
  }
}
```

### Step 2: Register the Extension

Make a POST request to register your extension:

```bash
curl -X POST https://your-clovalink-instance.com/api/extensions/register \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Extension",
    "slug": "my-extension",
    "manifest_url": "https://example.com/extension/manifest.json",
    "signature_algorithm": "hmac_sha256"
  }'
```

The response includes a `signing_key` that you'll use to verify webhook signatures:

```json
{
  "extension": { "id": "...", "name": "My Extension", ... },
  "signing_key": "abc123...",
  "message": "Extension registered successfully"
}
```

**Important:** Store the `signing_key` securely. You'll need it to verify webhook requests.

### Step 3: Install the Extension

Install for your tenant:

```bash
curl -X POST https://your-clovalink-instance.com/api/extensions/install/EXTENSION_ID \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "extension_id": "EXTENSION_ID",
    "permissions": ["read:files", "write:files"]
  }'
```

## Manifest Schema

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Human-readable name (max 255 chars) |
| `slug` | string | URL-safe identifier (lowercase, alphanumeric, hyphens) |
| `version` | string | Semantic version (e.g., "1.0.0") |
| `type` | enum | `ui`, `file_processor`, or `automation` |
| `permissions` | array | List of required permissions |

### UI Extension Fields

```json
{
  "ui": {
    "load_mode": "iframe",  // "iframe" or "esm"
    "sidebar": [...],
    "buttons": [...],
    "components": [...]
  }
}
```

#### Sidebar Items

```json
{
  "id": "unique-id",
  "name": "Display Name",
  "icon": "https://example.com/icon.svg",  // Optional
  "entrypoint": "https://example.com/sidebar.html",
  "order": 100  // Lower = higher in list
}
```

#### Buttons

```json
{
  "id": "unique-id",
  "name": "Button Label",
  "icon": "https://example.com/icon.svg",
  "location": "file_actions",  // Where to show the button
  "entrypoint": "https://example.com/action.html"
}
```

Button locations:
- `file_actions` - File action toolbar
- `toolbar` - Main toolbar
- `context_menu` - Right-click context menu

### File Processor Fields

```json
{
  "type": "file_processor",
  "permissions": ["file_processor:run", "read:files"],
  "webhook": "https://your-server.com/webhook/file-upload",
  "file_processor": {
    "file_types": ["pdf", "docx", "xlsx"],
    "max_file_size_mb": 100,
    "async_processing": true
  }
}
```

### Automation Fields

```json
{
  "type": "automation",
  "permissions": ["automation:run"],
  "webhook": "https://your-server.com/webhook/automation",
  "automation": {
    "default_cron": "0 0 * * *",
    "configurable": true,
    "config_schema": {
      "type": "object",
      "properties": {
        "option1": { "type": "string" }
      }
    }
  }
}
```

## Available Permissions

| Permission | Description |
|------------|-------------|
| `read:files` | Read file metadata and contents |
| `write:files` | Upload, modify, and delete files |
| `read:company` | Read company information and settings |
| `read:employees` | Read employee/user information |
| `automation:run` | Execute automation tasks on schedule |
| `file_processor:run` | Process files when uploaded |

## Webhook Integration

### Receiving Webhooks

For `file_processor` and `automation` extensions, ClovaLink sends webhook requests to your endpoint.

#### Request Headers

| Header | Description |
|--------|-------------|
| `X-ClovaLink-Signature` | HMAC or Ed25519 signature of the payload |
| `X-ClovaLink-Event` | Event type (e.g., `file_uploaded`, `automation_trigger`) |
| `X-ClovaLink-Extension-Id` | Your extension's UUID |
| `X-ClovaLink-Timestamp` | ISO 8601 timestamp |

#### File Upload Event Payload

```json
{
  "company_id": "uuid",
  "user_id": "uuid",
  "file_id": "uuid",
  "filename": "document.pdf",
  "content_type": "application/pdf",
  "size_bytes": 1234567,
  "event": "file_uploaded",
  "metadata": {},
  "timestamp": "2024-01-01T00:00:00Z"
}
```

#### Automation Event Payload

```json
{
  "company_id": "uuid",
  "extension_id": "uuid",
  "job_id": "uuid",
  "event": "automation_trigger",
  "config": {},
  "timestamp": "2024-01-01T00:00:00Z"
}
```

### Verifying Signatures

#### HMAC-SHA256

```python
import hmac
import hashlib

def verify_signature(payload: bytes, signature: str, secret: str) -> bool:
    expected = f"sha256={hmac.new(secret.encode(), payload, hashlib.sha256).hexdigest()}"
    return hmac.compare_digest(expected, signature)

# Usage
signature = request.headers['X-ClovaLink-Signature']
is_valid = verify_signature(request.body, signature, YOUR_SIGNING_KEY)
```

#### Ed25519

```python
from nacl.signing import VerifyKey
from nacl.encoding import HexEncoder

def verify_ed25519(payload: bytes, signature: str, public_key: str) -> bool:
    sig_hex = signature.removeprefix("ed25519=")
    sig_bytes = bytes.fromhex(sig_hex)
    key = VerifyKey(bytes.fromhex(public_key))
    try:
        key.verify(payload, sig_bytes)
        return True
    except:
        return False
```

## UI Extension Communication

### Iframe Extensions

Iframe extensions communicate via `postMessage`:

```javascript
// Listen for context from ClovaLink
window.addEventListener('message', (event) => {
  if (event.data.type === 'clovalink:context') {
    const { theme } = event.data.payload;
    // Apply theme, etc.
  }
});

// Send messages to ClovaLink
parent.postMessage({
  type: 'extension:navigate',
  payload: { path: '/files' }
}, '*');
```

Available message types:
- `extension:ready` - Signal that extension has loaded
- `extension:navigate` - Request navigation (relative paths only)
- `extension:resize` - Request panel resize

### ES Module Extensions

ES module extensions receive a `context` prop:

```javascript
export default function MyExtension({ context }) {
  const { extensionId, theme, api } = context;
  
  // Use API methods
  api.navigate('/files');
  api.showToast('Hello!', 'success');
  
  return <div>Extension Content</div>;
}
```

Context API:
- `extensionId` - Your extension's UUID
- `theme` - Current theme (`'light'` or `'dark'`)
- `api.navigate(path)` - Navigate to a path
- `api.showToast(message, type)` - Show a toast notification
- `api.getToken()` - Get auth token for API calls

## Best Practices

1. **Security**
   - Always verify webhook signatures
   - Use HTTPS for all endpoints
   - Don't store sensitive data in manifests

2. **Performance**
   - Keep iframe/ES module bundles small
   - Use lazy loading for heavy components
   - Respond to webhooks quickly (< 5 seconds)

3. **UX**
   - Match ClovaLink's design system
   - Support both light and dark themes
   - Handle loading and error states gracefully

4. **Reliability**
   - Implement retry logic for webhook failures
   - Use idempotent operations
   - Log errors for debugging

## API Reference

### Extension Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/extensions/register` | Register new extension |
| POST | `/api/extensions/install/:id` | Install extension |
| GET | `/api/extensions/list` | List registered extensions |
| GET | `/api/extensions/installed` | List installed extensions |
| POST | `/api/extensions/validate-manifest` | Validate manifest |
| GET | `/api/extensions/ui` | Get UI components |
| PUT | `/api/extensions/:id/settings` | Update settings |
| DELETE | `/api/extensions/:id` | Uninstall extension |

### Automation Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/extensions/:id/jobs` | List automation jobs |
| POST | `/api/extensions/:id/jobs` | Create automation job |
| POST | `/api/extensions/trigger/automation/:job_id` | Trigger job manually |

## Support

For questions or issues, contact support or open an issue in the repository.

