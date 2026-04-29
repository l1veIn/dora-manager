from __future__ import annotations

from typing import Any

import requests

from ._util import env_or_default, normalize_url


class ServiceError(RuntimeError):
    """Base error for dm-faasd service calls."""


class ServiceNotFoundError(ServiceError):
    """Raised when a service does not exist."""


class MethodNotFoundError(ServiceError):
    """Raised when a service method does not exist."""


class ServiceUnavailableError(ServiceError):
    """Raised when dm-faasd cannot serve the request right now."""


class Service:
    """HTTP client for dora-manager service functions."""

    def __init__(self, faasd_url: str | None = None, *, timeout: float = 5.0):
        self.faasd_url = normalize_url(
            faasd_url or env_or_default("DM_FAASD_URL", "http://127.0.0.1:5001")
        )
        self.timeout = timeout

    def invoke(
        self,
        service: str,
        method: str = "run",
        input: dict[str, Any] | None = None,
    ) -> Any:
        try:
            response = requests.post(
                f"{self.faasd_url}/fn/{service}/invoke",
                json={"method": method, "input": input or {}},
                timeout=self.timeout,
            )
        except requests.RequestException as exc:
            raise ServiceError(str(exc)) from exc
        if response.status_code == 200:
            return response.json()["output"]
        self._raise_for_response(response, service, method)

    def list(self) -> list[dict[str, Any]]:
        try:
            response = requests.get(f"{self.faasd_url}/fn", timeout=self.timeout)
        except requests.RequestException as exc:
            raise ServiceError(str(exc)) from exc
        if response.status_code == 200:
            return response.json()
        self._raise_for_response(response)

    def _raise_for_response(
        self,
        response: requests.Response,
        service: str | None = None,
        method: str | None = None,
    ) -> None:
        body = _json_or_empty(response)
        message = body.get("error") or response.text or f"HTTP {response.status_code}"

        if response.status_code == 404:
            raise ServiceNotFoundError(message)
        if response.status_code == 400 and body.get("code") == "method_not_found":
            raise MethodNotFoundError(message)
        if response.status_code == 503:
            raise ServiceUnavailableError(message)
        if service and method:
            raise ServiceError(f"{service}.{method} failed: {message}")
        raise ServiceError(message)


def _json_or_empty(response: requests.Response) -> dict[str, Any]:
    try:
        data = response.json()
    except ValueError:
        return {}
    return data if isinstance(data, dict) else {}
