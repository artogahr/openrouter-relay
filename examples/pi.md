# Pi example

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
