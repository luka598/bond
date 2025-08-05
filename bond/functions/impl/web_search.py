from io import BytesIO
from urllib.parse import quote

import html2text
import pycurl

from bond.config import GLOBAL_CONFIG
from bond.llm.interface import FunctionType


def web_search(query: str) -> str:
    if not GLOBAL_CONFIG.get("enable_web_search", False):
        return "Web search is disabled."

    curl = None
    body_buffer = BytesIO()
    header_buffer = BytesIO()

    try:
        url = "https://html.duckduckgo.com/html"
        post_data = f"q={quote(query)}"

        curl = pycurl.Curl()
        curl.setopt(curl.URL, url)
        curl.setopt(curl.POSTFIELDS, post_data.encode("utf-8"))
        curl.setopt(curl.POST, 1)

        curl.setopt(
            curl.HTTPHEADER,
            [
                "User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
                "Content-Type: application/x-www-form-urlencoded",
            ],
        )

        curl.setopt(curl.TIMEOUT, 10)
        curl.setopt(curl.WRITEDATA, body_buffer)
        curl.setopt(curl.HEADERFUNCTION, header_buffer.write)
        curl.setopt(curl.FOLLOWLOCATION, 1)
        curl.setopt(curl.MAXREDIRS, 5)

        curl.perform()

        status_code = curl.getinfo(pycurl.RESPONSE_CODE)
        if status_code >= 400:
            return f"Error searching for {query}: HTTP {status_code}"

        response_body_bytes = body_buffer.getvalue()
        response_headers_bytes = header_buffer.getvalue()
        headers_str = response_headers_bytes.decode("iso-8859-1")

        content_type = ""
        for line in headers_str.splitlines():
            if line.lower().startswith("content-type:"):
                content_type = line.split(":", 1)[1].strip()
                break

        encoding = "utf-8"
        if "charset=" in content_type.lower():
            parts = content_type.lower().split("charset=")
            if len(parts) > 1:
                encoding = parts[1].split(";")[0].strip()

        try:
            response_text = response_body_bytes.decode(encoding, errors="replace")
        except LookupError:
            response_text = response_body_bytes.decode("utf-8", errors="replace")

        if "text/html" in content_type.lower():
            h = html2text.HTML2Text()
            h.ignore_links = False
            markdown_content = h.handle(response_text)
            return markdown_content[713:]
        else:
            return response_text

    except pycurl.error as e:
        err_code = e.args[0]
        err_msg = e.args[1]
        return f"Error searching for {query} (pycurl error {err_code}): {err_msg}"
    except Exception as e:
        return f"An unexpected error occurred: {e}"
    finally:
        if curl:
            curl.close()
        body_buffer.close()
        header_buffer.close()


FUNCTION = (
    FunctionType(
        "web_search",
        "Searches for content for a specified query.",
        [
            FunctionType.Param(
                "query",
                "string",
                "A string used for searching.",
            ),
        ],
    ),
    web_search,
)
