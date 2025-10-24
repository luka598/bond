from bond.lib.llm.impl.oai_old import OAILLM

class GeminiLLM(OAILLM):
    ENDPOINT = "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"