# OpenRouter relay

Use OpenRouter models from a local coding agent with your Apify account. This Standby Actor streams OpenAI-compatible requests through [OpenRouter](https://apify.com/apify/openrouter). Models run remotely. Your agent sends prompts and any file content it includes to the selected model.

Apify authenticates each request to the Standby URL. Each caller gets a separate Standby run with their own injected `APIFY_TOKEN`. The relay uses that run token for the upstream request. Each caller pays for their own Standby compute and OpenRouter usage.

Supported routes:

| Method | Path |
| --- | --- |
| POST | `/api/v1/chat/completions` |
| POST | `/api/v1/responses` |
| POST | `/api/v1/messages` |
| GET | `/api/v1/models` |

The request and response bodies follow the corresponding OpenRouter API formats. The relay passes model IDs unchanged, so any model accepted by OpenRouter can be requested. The upstream Actor's limits still apply, including its current 2,048 output-token cap for Chat and Responses requests. Use the Standby endpoint to send requests; starting a run in Apify Console produces no stored output.

## Use with Pi

1. Install [Pi](https://pi.dev) and the [Apify CLI](https://docs.apify.com/cli/). Run `apify login`.
2. Add this to `~/.pi/agent/models.json`:

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

3. Start Pi and choose an `openrouter` model in `/model`.

This keeps Pi's built-in OpenRouter model catalog and model picker. You can switch models without editing this file. Pi executes `apify auth token` when it needs a credential, so the token stays out of the configuration file. A stored OpenRouter credential in Pi's `auth.json` takes priority over the `apiKey` above; remove that credential if Pi sends the wrong token. If the Apify CLI is unavailable, set `APIFY_TOKEN` in Pi's environment and use `"apiKey": "$APIFY_TOKEN"` instead.

## HTTP example

```sh
APIFY_TOKEN=$(apify auth token)
STANDBY_URL=https://artogahr--openrouter-relay.apify.actor
curl "$STANDBY_URL/api/v1/chat/completions" \
  -H "Authorization: Bearer $APIFY_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"model":"openrouter/auto","messages":[{"role":"user","content":"Hello"}],"stream":true}'
```

## Develop

The [source code](https://github.com/artogahr/openrouter-relay) is an MIT-licensed standalone Nix flake. Run `nix develop` for the Rust toolchain, then `cargo test --locked` and `cargo clippy --all-targets -- -D warnings`. Run `nix build` to build the relay with Nix. The tests use a local mock upstream and spend no model tokens. Pushes to `main` run Rust checks in GitHub Actions and start an Apify build from this repository.
