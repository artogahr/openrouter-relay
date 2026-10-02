## Use OpenRouter models from Pi and other local agents

This Actor lets a local AI client use OpenRouter models with an Apify API token. It forwards supported API requests to [Apify's OpenRouter Actor](https://apify.com/apify/openrouter) and streams the response back. The models run remotely; you do not need a local GPU.

Use the Standby URL in your client's provider settings. There is no Actor input to configure. Apify starts a separate Standby run for each caller and injects that caller's `APIFY_TOKEN`; the relay uses it for the upstream request. Your prompts, tool results, and any file content your client sends go to the selected model provider through OpenRouter.

### Supported API routes

| Method | Path | Purpose |
| --- | --- | --- |
| POST | `/api/v1/chat/completions` | OpenAI chat completions, including streaming |
| POST | `/api/v1/responses` | OpenAI Responses API, including streaming |
| POST | `/api/v1/messages` | Anthropic-format messages, including streaming |
| GET | `/api/v1/models` | List models available through the upstream Actor |

The relay passes JSON bodies and model IDs through unchanged. It replaces the caller's authorization with the Standby run token. Other routes return `404`. Upstream errors and rate limits reach the client with their status codes.

## Set up Pi

1. Install [Pi](https://pi.dev) and the [Apify CLI](https://docs.apify.com/cli/). Run `apify login`.
2. Add this provider to `~/.pi/agent/models.json`:

```json
{
  "providers": {
    "openrouter": {
      "baseUrl": "https://artogahr--openrouter-relay.apify.actor/api/v1",
      "apiKey": "!apify auth token"
    }
  }
}
```

3. Start Pi and choose a model from its `openrouter` provider with `/model`.

Pi keeps its OpenRouter model catalog and model picker, so switching models needs no relay change. Pi runs `apify auth token` when it needs a credential; the token is not stored in `models.json`. If Pi has an OpenRouter credential in `~/.pi/agent/auth.json`, that credential takes priority. Remove it if requests authenticate with the wrong token. You can also set `APIFY_TOKEN` in Pi's environment and use `"apiKey": "$APIFY_TOKEN"`.

## Send an HTTP request

```sh
APIFY_TOKEN=$(apify auth token)
curl 'https://artogahr--openrouter-relay.apify.actor/api/v1/chat/completions' \
  -H "Authorization: Bearer $APIFY_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"model":"openrouter/auto","messages":[{"role":"user","content":"Hello"}],"stream":true}'
```

The response is the upstream JSON response or server-sent event stream. For example, a non-streaming chat response contains `choices[0].message.content`. The [Endpoints tab](https://apify.com/artogahr/openrouter-relay) lets you inspect and try each route in a browser. Starting the Actor as a normal Console run checks the upstream model catalog and writes one diagnostic dataset item; it does not start the HTTP relay.

## Pricing and limits

Each caller pays for their own Standby run's Apify compute and for model use charged by `apify/openrouter`. This relay adds no pay-per-event charge. The Standby run stops after its idle timeout, so an occasional request may wait for a new run to start. Check the [upstream pricing and limits](https://apify.com/apify/openrouter) before choosing a model. In particular, its Chat and Responses routes currently cap output at 2,048 tokens. Model availability and provider limits can change.

## Troubleshooting and support

- `401` or `403`: check that your client sends an Apify API token to the Standby URL. In Pi, also check for a stored OpenRouter credential in `auth.json`.
- `404`: use one of the four supported paths above and include `/api/v1` in the base URL.
- `429` or another upstream error: inspect the response body and your Apify usage limits. The relay preserves upstream status codes.
- A long first request: Apify may be starting a Standby run. Later requests to an active run should avoid that startup wait.

For a relay bug or integration question, [open a GitHub issue](https://github.com/artogahr/openrouter-relay/issues). For Apify account or billing questions, use [Apify support](https://help.apify.com/).

## Develop

The [source code](https://github.com/artogahr/openrouter-relay) is MIT-licensed and packaged as a standalone Nix flake. Run `nix develop`, then `cargo test --locked` and `cargo clippy --all-targets -- -D warnings`. Run `nix build` to build with Nix. Tests use a local mock upstream and spend no model tokens. Pushes to `main` run Rust checks in GitHub Actions and start an Apify build from this repository.
