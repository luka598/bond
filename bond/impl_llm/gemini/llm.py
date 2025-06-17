import typing as T
import json
import requests
import time

from bond.config import Config
from bond.llm import (
    LLM,
    MSG_t,
    ROLE_t,
    TextMsg,
    FunctionResultMsg,
    FunctionType,
    FunctionCallMsg,
)


def convert_msg_to_gemini_format(
    msg: MSG_t, is_first_user_message_after_system: bool = False
) -> T.Optional[T.Dict[str, T.Any]]:
    if isinstance(msg, TextMsg):
        if msg.role == "system":
            return None
        elif msg.role == "user":
            return {"role": "user", "parts": [{"text": msg.data}]}
        elif msg.role == "llm":
            return {"role": "model", "parts": [{"text": msg.data}]}
        else:
            raise RuntimeError(f"Unknown role in TextMsg: {msg.role}")
    elif isinstance(msg, FunctionCallMsg):
        return {
            "role": "model",
            "parts": [{"functionCall": {"name": msg.name, "args": msg.params}}],
        }
    elif isinstance(msg, FunctionResultMsg):
        return {
            "role": "user",
            "parts": [
                {
                    "functionResponse": {
                        "name": msg.name,
                        "response": {"content": msg.data},
                    }
                }
            ],
        }
    else:
        print(f"Unknown message type: {type(msg)}")
        return {
            "role": "user",
            "parts": [
                {"text": f"[Internal Error: Unknown message type {type(msg).__name__}]"}
            ],
        }


def convert_function_to_gemini_format(f: FunctionType) -> T.Dict[str, T.Any]:
    properties = {}
    for name, type_str, desc in f.params:
        gemini_type = type_str.upper()
        if gemini_type not in [
            "STRING",
            "NUMBER",
            "INTEGER",
            "BOOLEAN",
            "ARRAY",
            "OBJECT",
        ]:
            print(
                f"Warning: Unknown parameter type '{type_str}' for function '{f.name}'. Defaulting to STRING."
            )
            gemini_type = "STRING"
        properties[name] = {"type": gemini_type, "description": desc}
    required_params = [
        name
        for (name, _, _) in f.params
        if name not in getattr(f, "optional_params", [])
    ]
    return {
        "name": f.name,
        "description": f.description,
        "parameters": {
            "type": "OBJECT",
            "properties": properties,
            "required": required_params,
        },
    }


class GeminiLLM(LLM):
    BASE_ENDPOINT = "https://generativelanguage.googleapis.com/v1beta/models"

    def __init__(self, config: Config) -> None:
        super().__init__(config)
        self.api_key = self.config["api_key"]
        self.model_name = self.config["model_name"]
        self.endpoint = (
            f"{self.BASE_ENDPOINT}/{self.model_name}:generateContent?key={self.api_key}"
        )
        self.HEADERS = {"Content-Type": "application/json"}

        # Retry and timeout configurations
        self.max_retries: int = self.config.get("max_retries", 3)
        self.initial_retry_delay_seconds: float = self.config.get(
            "initial_retry_delay_seconds", 5.0
        )
        self.request_timeout_seconds: float = self.config.get(
            "request_timeout_seconds", 60.0
        )

    def _parse_retry_delay(
        self, error_response: requests.Response
    ) -> T.Optional[float]:
        try:
            error_json = error_response.json()
            if "error" in error_json and "details" in error_json["error"]:
                for detail in error_json["error"]["details"]:
                    if (
                        detail.get("@type")
                        == "type.googleapis.com/google.rpc.RetryInfo"
                    ):
                        retry_after_str = detail.get("retryDelay")
                        if retry_after_str and retry_after_str.endswith("s"):
                            try:
                                delay_from_api = float(retry_after_str[:-1])
                                return max(delay_from_api, 1.0)  # Ensure at least 1s
                            except ValueError:
                                print(
                                    f"Could not parse API retryDelay value: {retry_after_str}."
                                )
                        break
        except json.JSONDecodeError:
            print(
                f"Could not parse JSON from 429 error response text: {error_response.text}"
            )
        except Exception as e:
            print(f"Unexpected error parsing retry delay: {e}")
        return None

    def _make_request_with_retry(
        self, payload: T.Dict[str, T.Any]
    ) -> T.Optional[requests.Response]:
        retries = 0
        last_exception: T.Optional[Exception] = None

        while retries <= self.max_retries:
            try:
                response = requests.post(
                    self.endpoint,
                    headers=self.HEADERS,
                    json=payload,
                    timeout=self.request_timeout_seconds,
                )
                response.raise_for_status()
                return response
            except requests.exceptions.HTTPError as e:
                last_exception = e
                if e.response is not None and e.response.status_code == 429:
                    if retries == self.max_retries:
                        print(
                            f"Max retries ({self.max_retries}) reached for 429 error. Failing."
                        )
                        break

                    retry_delay_seconds = self._parse_retry_delay(e.response)
                    if retry_delay_seconds is None:
                        retry_delay_seconds = self.initial_retry_delay_seconds * (
                            2**retries
                        )
                        print(f"Using default backoff: {retry_delay_seconds:.2f}s.")
                    else:
                        print(f"API suggests retry after {retry_delay_seconds:.2f}s.")

                    print(
                        f"Rate limit hit (429). Attempt {retries + 1}/{self.max_retries + 1}. Retrying in {retry_delay_seconds:.2f} seconds..."
                    )
                    time.sleep(retry_delay_seconds)
                    retries += 1
                else:
                    print(
                        f"Request failed with non-retryable HTTPError (status {e.response.status_code if e.response is not None else 'N/A'}): {e}"
                    )
                    if e.response is not None:
                        print(f"Response content: {e.response.text}")
                    return None
            except requests.exceptions.Timeout as e:
                last_exception = e
                if retries == self.max_retries:
                    print(
                        f"Max retries ({self.max_retries}) reached after Timeout. Failing."
                    )
                    break
                retry_delay_seconds = self.initial_retry_delay_seconds * (2**retries)
                print(
                    f"Request timed out. Attempt {retries + 1}/{self.max_retries + 1}. Retrying in {retry_delay_seconds:.2f} seconds..."
                )
                time.sleep(retry_delay_seconds)
                retries += 1
            except requests.exceptions.RequestException as e:
                last_exception = e
                if retries == self.max_retries:
                    print(
                        f"Max retries ({self.max_retries}) reached after {type(e).__name__}. Failing."
                    )
                    break
                retry_delay_seconds = self.initial_retry_delay_seconds * (2**retries)
                print(
                    f"General request error ({type(e).__name__}). Attempt {retries + 1}/{self.max_retries + 1}. Retrying in {retry_delay_seconds:.2f} seconds..."
                )
                time.sleep(retry_delay_seconds)
                retries += 1

        print(f"Failed to send request after {retries} attempts.")
        if last_exception:
            print(f"Last error: {type(last_exception).__name__}: {last_exception}")
            if (
                hasattr(last_exception, "response")
                and last_exception.response is not None
            ):
                try:
                    error_text = last_exception.response.text
                    try:
                        error_json_parsed = json.loads(error_text)
                        print(
                            f"Last error response content (JSON):\n{json.dumps(error_json_parsed, indent=2)}"
                        )
                    except json.JSONDecodeError:
                        print(f"Last error response content (text):\n{error_text}")
                except Exception as read_exc:
                    print(f"Could not read last error response content: {read_exc}")
        return None

    def _process_response_json(self, j: T.Dict[str, T.Any]) -> T.List[MSG_t]:
        if not j.get("candidates"):
            if "promptFeedback" in j:
                feedback = j["promptFeedback"]
                print(
                    f"Warning: No candidates returned. Prompt feedback: {json.dumps(feedback, indent=2)}"
                )
                if feedback.get("blockReason"):
                    print(
                        f"Request may have been blocked. Reason: {feedback.get('blockReason')}"
                    )
            else:
                print(
                    f"Warning: No candidates in response. Full response: {json.dumps(j, indent=2)}"
                )
            return []

        candidate = j["candidates"][0]
        content = candidate.get("content")

        if not content or not content.get("parts"):
            finish_reason = candidate.get("finishReason")
            safety_ratings = candidate.get("safetyRatings")
            print(
                f"Warning: No content or parts in candidate. Finish Reason: {finish_reason}. Safety Ratings: {safety_ratings}"
            )
            print(f"Full candidate: {json.dumps(candidate, indent=2)}")
            return []

        result_messages: T.List[MSG_t] = []
        for part in content["parts"]:
            if "text" in part:
                result_messages.append(TextMsg(role="llm", data=part["text"]))
            elif "functionCall" in part:
                fcall = part["functionCall"]
                params = fcall.get("args", {})
                result_messages.append(
                    FunctionCallMsg(name=fcall["name"], params=params)
                )
            else:
                print(f"Unknown part structure in response: {part}")
        return result_messages

    def send(
        self, messages: T.List[MSG_t], functions: T.List[FunctionType]
    ) -> T.List[MSG_t]:
        gemini_contents = []
        system_instruction_parts = []
        processed_messages = []

        for msg in messages:
            if isinstance(msg, TextMsg) and msg.role == "system":
                system_instruction_parts.append({"text": msg.data})
            else:
                processed_messages.append(msg)

        is_first_user_message_after_system = (
            bool(system_instruction_parts)
            and len(processed_messages) > 0
            and isinstance(processed_messages[0], TextMsg)
            and processed_messages[0].role == "user"
        )

        for i, msg in enumerate(processed_messages):
            is_current_message_first_user_after_system = (
                i == 0 and is_first_user_message_after_system
            )
            gemini_msg = convert_msg_to_gemini_format(
                msg, is_current_message_first_user_after_system
            )
            if gemini_msg:
                gemini_contents.append(gemini_msg)

        payload: T.Dict[str, T.Any] = {"contents": gemini_contents}
        if system_instruction_parts:
            payload["system_instruction"] = {
                "role": "system",
                "parts": system_instruction_parts,
            }
        if functions:
            payload["tools"] = [
                {
                    "function_declarations": [
                        convert_function_to_gemini_format(f) for f in functions
                    ]
                }
            ]
            payload["tool_config"] = {"function_calling_config": {"mode": "AUTO"}}

        response = self._make_request_with_retry(payload)

        if response is None:
            print("Failed to get a response from Gemini API after all retries.")
            return []

        try:
            response_json = response.json()
        except json.JSONDecodeError:
            print(f"Failed to decode JSON response: {response.text}")
            return []

        return self._process_response_json(response_json)
