import html2text
import requests

from bond.lib.functions.interface import FunctionType, Function


def web_fetch(url: str, prompt: str) -> dict:
    try:
        response = requests.get(url, timeout=10)
        response.raise_for_status()
        content_type = response.headers.get("Content-Type", "")
        if "text/html" in content_type:
            h = html2text.HTML2Text()
            h.ignore_links = False
            markdown_content = h.handle(response.text)
            output = markdown_content
        else:
            output = response.text
        return {"success": True, "output": output, "error": ""}

    except requests.exceptions.RequestException as e:
        return {"success": False, "output": "", "error": f"Error fetching URL {url}: {e}"}
    except Exception as e:
        return {"success": False, "output": "", "error": f"An unexpected error occurred: {e}"}


class WebFetchFunction(Function):
    FUNCTION_t = FunctionType(
        "web_fetch",
        "Fetches content from a specified URL and converts HTML to Markdown. Takes a URL and a prompt as input.",
        [
            FunctionType.ParamLiteral("url", "string", "The URL to fetch."),
            FunctionType.ParamLiteral(
                "prompt",
                "string",
                "A prompt for the AI model to process the content (not directly used by the tool, but for context).",
            ),
        ],
    )
    CALLABLE = web_fetch