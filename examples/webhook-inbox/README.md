# Yaiko Webhook Inbox Example

This example demonstrates signed webhook events using Yaiko’s `WebhookEvent` and `WebhookVerifier` modules. It serializes a typed event, signs the timestamped body, verifies it, and reports that replay protection is active.

## Run

```bash
cd examples/webhook-inbox
yaiko doctor
yaiko build
yaiko run
```

For a production HTTP endpoint, pass the raw request body and the received signature header to the same verifier before decoding or dispatching the event.
