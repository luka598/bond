import html2text
import requests

from bond.lib.functions.interface import FunctionType, Function

ENABLE_WEB_SEARCH = True


def web_search(query: str) -> dict:
    if not ENABLE_WEB_SEARCH:
        return {"success": False, "output": "", "error": "Web search is disabled."}

    try:
        url = "https://html.duckduckgo.com/html"
        payload = {"q": query}
        headers = {
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
            "Content-Type": "application/x-www-form-urlencoded",
        }

        response = requests.post(url, data=payload, headers=headers, timeout=10)
        if response.status_code >= 400:
            return {"success": False, "output": "", "error": f"Error searching for {query}: HTTP {response.status_code}"}

        content_type = response.headers.get("Content-Type", "")
        encoding = response.encoding or "utf-8"
        if "charset=" in content_type.lower():
            parts = content_type.lower().split("charset=")
            if len(parts) > 1:
                encoding = parts[1].split(";")[0].strip()

        response_text = response.content.decode(encoding, errors="replace")

        if "text/html" in content_type.lower():
            h = html2text.HTML2Text()
            h.ignore_links = False
            markdown_content = h.handle(response_text)
            output = markdown_content[713:]
        else:
            output = response_text

        return {"success": True, "output": output, "error": ""}

    except requests.RequestException as e:
        return {"success": False, "output": "", "error": f"Error searching for {query} (requests error): {e}"}
    except Exception as e:
        return {"success": False, "output": "", "error": f"An unexpected error occurred: {e}"}


class WebSearchFunction(Function):
    FUNCTION_t = FunctionType(
        "web_search",
        "Searches for content for a specified query.",
        [
            FunctionType.ParamLiteral("query", "string", "A string used for searching."),
        ],
    )
    CALLABLE = web_search