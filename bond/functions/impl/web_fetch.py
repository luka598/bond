import requests
from bond.llm.interface import FunctionType
import html2text


def web_fetch(url: str, prompt: str) -> str:
    try:
        response = requests.get(url, timeout=10)
        response.raise_for_status()

        content_type = response.headers.get("Content-Type", "")
        if "text/html" in content_type:
            h = html2text.HTML2Text()
            h.ignore_links = False
            markdown_content = h.handle(response.text)
            return markdown_content
        else:
            return response.text

    except requests.exceptions.RequestException as e:
        return f"Error fetching URL {url}: {e}"
    except Exception as e:
        return f"An unexpected error occurred: {e}"


FUNCTION = (
    FunctionType(
        "web_fetch",
        "Fetches content from a specified URL and converts HTML to Markdown. Takes a URL and a prompt as input.",
        [
            FunctionType.Param("url", "string", "The URL to fetch."),
            FunctionType.Param(
                "prompt",
                "string",
                "A prompt for the AI model to process the content (not directly used by the tool, but for context).",
            ),
        ],
    ),
    web_fetch,
)
