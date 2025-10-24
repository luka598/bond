from bond.lib.llm.impl.openai_old import OpenAILLM

class GeminiLLM(OpenAILLM):
    ENDPOINT = "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"