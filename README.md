<p align="center"><strong>LocalDex</strong> is a Codex-compatible coding agent that runs locally on your computer.
<p align="center">
  <img src="https://github.com/openai/codex/blob/main/.github/codex-cli-splash.png" alt="Codex CLI splash" width="80%" />
</p>
</br>
If you want Codex in your code editor (VS Code, Cursor, Windsurf), <a href="https://developers.openai.com/codex/ide">install in your IDE.</a>
</br>If you want the desktop app experience, run <code>codex app</code> or visit <a href="https://chatgpt.com/codex?app-landing-page=true">the Codex App page</a>.
</br>If you are looking for the <em>cloud-based agent</em> from OpenAI, <strong>Codex Web</strong>, go to <a href="https://chatgpt.com/codex">chatgpt.com/codex</a>.</p>

---

## Quickstart

### Installing and running LocalDex

The supported release currently targets Linux x86_64. The installer downloads
only the LocalDex release, installs the `codex` command and its Code Mode
companion, and leaves the existing `CODEX_HOME` auth, configuration, and
session files in place:

```shell
curl -fsSL https://github.com/ModelCloud/LocalDex/releases/latest/download/install-localdex.sh | sh
```

To pin a release, pass `--release VERSION` to the downloaded installer. Each
upgrade keeps prior versioned binaries under `CODEX_HOME/packages/standalone/`
so rollback can switch the `current` symlink back to the previous release.
Local model endpoints are not contacted during installation or upgrade.

To roll back, inspect the retained release directories and repoint `current`:

```shell
CODEX_HOME_DIR="${CODEX_HOME:-$HOME/.codex}"
ls "$CODEX_HOME_DIR/packages/standalone/releases"
ln -sfn "$CODEX_HOME_DIR/packages/standalone/releases/<previous-release>" \
  "$CODEX_HOME_DIR/packages/standalone/current"
```

### Configure named OpenAI-compatible providers

LocalDex keeps the built-in `openai` provider and its normal ChatGPT/API-key
authentication. Add custom providers by name in `CODEX_HOME/config.toml`; each
provider has its own endpoint and bearer-token environment variable:

```toml
model = "gpt-6-sol"
model_provider = "openai"

[model_providers.dsv41]
name = "DSV4.1 endpoint"
base_url = "http://10.0.13.33:2120/v1"
env_key = "DSV41_BEARER_TOKEN"
wire_api = "responses"
requires_openai_auth = false

[profiles.dsv41]
model = "QB/DSV4.1-Flash"
model_provider = "dsv41"
```

Set `DSV41_BEARER_TOKEN` in the environment before launching LocalDex, then
select the `dsv41` profile with `codex --profile dsv41`. The bearer value stays
out of TOML, command-line arguments, and LocalDex logs. Repeat the provider and
profile sections with distinct names for additional endpoints. Do not set a
global `openai_base_url` when you need official OpenAI models and custom models
to share the same session.

For Omnigent, put provider definitions in
`~/.local/share/localdex/config.toml` and model-to-provider mappings in
`~/.local/share/localdex/models.toml`. This keeps the registrations out of
Omnigent's global provider catalog while letting one LocalDex harness offer
official and custom models together:

```toml
# ~/.local/share/localdex/config.toml
[model_providers.dsv41]
name = "DSV4.1 endpoint"
base_url = "http://10.0.13.33:2120/v1"
env_key = "DSV41_BEARER_TOKEN"
wire_api = "responses"
requires_openai_auth = false

[model_providers.lab]
name = "Lab endpoint"
base_url = "https://models.example/v1"
env_key = "LAB_BEARER_TOKEN"
wire_api = "responses"
requires_openai_auth = false
```

```toml
# ~/.local/share/localdex/models.toml
[models."QB/DSV4.1-Flash"]
provider = "dsv41"
display_name = "DSV4.1 Flash"
discover_capabilities = true

[models."lab/coder-32b"]
provider = "lab"
display_name = "Lab Coder 32B"
```

Export the named bearer variables in the Omnigent host service environment.
Only models with their configured bearer variable present are offered as
routable; selecting one routes to its named provider. Official Codex models
continue to use the normal ChatGPT login and OpenAI endpoints. Installation and
upgrade never contact or validate a model endpoint.

<details>
<summary>You can also go to the <a href="https://github.com/openai/codex/releases/latest">latest GitHub Release</a> and download the appropriate binary for your platform.</summary>

Each GitHub Release contains many executables, but in practice, you likely want one of these:

- macOS
  - Apple Silicon/arm64: `codex-aarch64-apple-darwin.tar.gz`
  - x86_64 (older Mac hardware): `codex-x86_64-apple-darwin.tar.gz`
- Linux
  - x86_64: `codex-x86_64-unknown-linux-musl.tar.gz`
  - arm64: `codex-aarch64-unknown-linux-musl.tar.gz`

Each archive contains a single entry with the platform baked into the name (e.g., `codex-x86_64-unknown-linux-musl`), so you likely want to rename it to `codex` after extracting it.

</details>

### Using Codex with your ChatGPT plan

Run `codex` and select **Sign in with ChatGPT**. We recommend signing into your ChatGPT account to use Codex as part of your Plus, Pro, Business, Edu, or Enterprise plan. [Learn more about what's included in your ChatGPT plan](https://help.openai.com/en/articles/11369540-codex-in-chatgpt).

You can also use Codex with an API key, but this requires [additional setup](https://developers.openai.com/codex/auth#sign-in-with-an-api-key).

## Docs

- [**Codex Documentation**](https://developers.openai.com/codex)
- [**Contributing**](./docs/contributing.md)
- [**Installing & building**](./docs/install.md)
- [**Open source fund**](./docs/open-source-fund.md)

This repository is licensed under the [Apache-2.0 License](LICENSE).
