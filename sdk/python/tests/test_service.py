from __future__ import annotations

import pytest
import requests

from dm import (
    MethodNotFoundError,
    Service,
    ServiceError,
    ServiceNotFoundError,
    ServiceUnavailableError,
)


class Response:
    def __init__(self, data, status_code=200, text=""):
        self._data = data
        self.status_code = status_code
        self.text = text or str(data)

    def json(self):
        if isinstance(self._data, BaseException):
            raise self._data
        return self._data


def test_service_reads_env(monkeypatch):
    monkeypatch.setenv("DM_FAASD_URL", "http://faasd.local/")

    service = Service()

    assert service.faasd_url == "http://faasd.local"


def test_invoke_posts_request_and_returns_output(monkeypatch):
    calls = {}

    def fake_post(url, json, timeout):
        calls.update(url=url, json=json, timeout=timeout)
        return Response({"output": {"result": 18}})

    monkeypatch.setattr("dm._service.requests.post", fake_post)

    output = Service(faasd_url="http://faasd").invoke(
        "add", method="sum", input={"x": 7, "y": 11}
    )

    assert output == {"result": 18}
    assert calls["url"] == "http://faasd/fn/add/invoke"
    assert calls["json"] == {"method": "sum", "input": {"x": 7, "y": 11}}
    assert calls["timeout"] == 5.0


def test_invoke_defaults_empty_input(monkeypatch):
    monkeypatch.setattr(
        "dm._service.requests.post",
        lambda url, json, timeout: Response({"output": json["input"]}),
    )

    assert Service().invoke("ping") == {}


@pytest.mark.parametrize(
    ("status", "body", "expected"),
    [
        (404, {"error": "function not found"}, ServiceNotFoundError),
        (
            400,
            {"error": "method 'x' not found", "code": "method_not_found"},
            MethodNotFoundError,
        ),
        (503, {"error": "busy"}, ServiceUnavailableError),
        (500, {"error": "boom"}, ServiceError),
    ],
)
def test_invoke_errors(monkeypatch, status, body, expected):
    monkeypatch.setattr(
        "dm._service.requests.post",
        lambda url, json, timeout: Response(body, status_code=status),
    )

    with pytest.raises(expected):
        Service().invoke("svc", method="x")


def test_list_returns_functions(monkeypatch):
    functions = [{"id": "add", "methods": [{"name": "run"}]}]
    monkeypatch.setattr(
        "dm._service.requests.get",
        lambda url, timeout: Response(functions),
    )

    assert Service(faasd_url="http://faasd").list() == functions


def test_list_error(monkeypatch):
    monkeypatch.setattr(
        "dm._service.requests.get",
        lambda url, timeout: Response({"error": "down"}, status_code=500),
    )

    with pytest.raises(ServiceError, match="down"):
        Service().list()


def test_request_exception_becomes_service_error(monkeypatch):
    def fake_post(url, json, timeout):
        raise requests.Timeout("timed out")

    monkeypatch.setattr("dm._service.requests.post", fake_post)

    with pytest.raises(ServiceError, match="timed out"):
        Service().invoke("svc")
