from bond.lib.functions.base import Function, FunctionResult

class WebSearchFunctionResult(FunctionResult):
    """The result of a web search operation."""

    @property
    def search_results(self) -> list:
        """The content that was viewed."""
        return self.value

    @property
    def query(self) -> str:
        """The query that was used for the search."""
        return self.args.query

    def __repr__(self) -> str:
        return f"WebSearch for '{self.query}' returned {len(self.search_results)} results"

class WebSearch(Function):
    """Search the web for a query."""

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.query = kwargs.get("query")

    @property
    def result_class(self):
        return WebSearchFunctionResult

    @classmethod
    def args_schema(cls) -> dict:
        return {
            "query": {"type": "string", "description": "The search query."},
        }

    @property
    def args(self) -> dict:
        return {"query": self.query}

    def __repr__(self) -> str:
        return f"WebSearch(query='{self.query}')"