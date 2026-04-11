# Bond

## Example config

`~/.config/bond/config.toml`:

```
model="minimax/minimax-m2.5"
provider_name="openrouter-default"

[[provider]]
name="openrouter-default"
backend="openai"
api_base=""
api_key="<API_KEY>"
extra = {providers="inceptron/fp8"}

[[provider]]
name="openai-default"
backend="openai"
api_base="https://api.openai.com/v1/responses"
api_key="<API_KEY>"
```