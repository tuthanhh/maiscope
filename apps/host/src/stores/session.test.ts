import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { SignInError, useSessionStore } from "./session";

// The session store's contract with the API (contract §4) and with itself:
// what survives a failure, what a race may not undo, where sign-in lands.
// The network is a queue of canned answers keyed by URL path.

const USER = { id: "u1", login: "tuthanhh", avatarUrl: null, role: "user" as const };

type Answer = { status: number; body?: unknown } | "network-error";
let answers: Record<string, Answer[]>;
let calls: string[];

function json(status: number, body: unknown): Response {
  return new Response(body === undefined ? null : JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function route(path: string, ...queue: Answer[]): void {
  answers[path] = queue;
}

beforeEach(() => {
  setActivePinia(createPinia());
  localStorage.clear();
  sessionStorage.clear();
  answers = {};
  calls = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL) => {
      const path = new URL(String(input)).pathname.replace(/^\/api/, "");
      calls.push(path);
      const next = answers[path]?.shift();
      if (!next) throw new Error(`unexpected request: ${path}`);
      if (next === "network-error") throw new TypeError("Failed to fetch");
      return json(next.status, next.body);
    }),
  );
});

describe("restore", () => {
  it("does nothing without a stored session", async () => {
    const session = useSessionStore();
    await session.restore();
    expect(session.ready).toBe(true);
    expect(session.signedIn).toBe(false);
    expect(calls).toEqual([]);
  });

  it("restores a stored session through one refresh and /me", async () => {
    localStorage.setItem("mv:refresh-token", "rt-1");
    route("/auth/refresh", { status: 200, body: { token: "jwt", refreshToken: "rt-2" } });
    route("/auth/me", { status: 200, body: USER });
    const session = useSessionStore();
    await session.restore();
    expect(session.user).toEqual(USER);
    expect(localStorage.getItem("mv:refresh-token")).toBe("rt-2");
  });

  it("forgets a session the API refuses", async () => {
    localStorage.setItem("mv:refresh-token", "rt-dead");
    route("/auth/refresh", { status: 401, body: { error: "unauthorized", message: "x" } });
    const session = useSessionStore();
    await session.restore();
    expect(session.signedIn).toBe(false);
    expect(session.unverified).toBe(false);
    expect(localStorage.getItem("mv:refresh-token")).toBeNull();
  });

  it.each([
    ["offline", "network-error" as const],
    ["a server error", { status: 503, body: { error: "x", message: "down" } }],
    ["rate limiting", { status: 429, body: { error: "rate_limited", message: "x" } }],
  ])("keeps a stored session it could not check (%s)", async (_, answer) => {
    localStorage.setItem("mv:refresh-token", "rt-1");
    route("/auth/refresh", answer);
    const session = useSessionStore();
    await session.restore();
    expect(session.signedIn).toBe(false);
    expect(session.unverified).toBe(true);
    expect(localStorage.getItem("mv:refresh-token")).toBe("rt-1");
  });

  it("does not store a malformed token body", async () => {
    localStorage.setItem("mv:refresh-token", "rt-1");
    route("/auth/refresh", { status: 200, body: { token: "jwt" } });
    const session = useSessionStore();
    await session.restore();
    expect(localStorage.getItem("mv:refresh-token")).toBe("rt-1");
    expect(session.unverified).toBe(true);
  });
});

describe("refresh", () => {
  it("runs once for concurrent requests: refresh tokens rotate", async () => {
    localStorage.setItem("mv:refresh-token", "rt-1");
    route("/auth/refresh", { status: 200, body: { token: "jwt", refreshToken: "rt-2" } });
    route("/auth/me", { status: 200, body: USER }, { status: 200, body: USER });
    const session = useSessionStore();
    await Promise.all([
      session.authFetch("http://localhost:3000/api/auth/me"),
      session.authFetch("http://localhost:3000/api/auth/me"),
    ]);
    expect(calls.filter((c) => c === "/auth/refresh")).toHaveLength(1);
  });

  it("cannot undo a sign-out that happened while it was in flight", async () => {
    localStorage.setItem("mv:refresh-token", "rt-1");
    let release!: () => void;
    const gate = new Promise<void>((resolve) => (release = resolve));
    vi.mocked(fetch).mockImplementationOnce(async () => {
      await gate;
      return json(200, { token: "jwt", refreshToken: "rt-2" });
    });
    const session = useSessionStore();
    const pending = session.authFetch("http://localhost:3000/api/auth/me").catch((e) => e);
    session.signOut();
    release();
    await pending;
    expect(localStorage.getItem("mv:refresh-token")).toBeNull();
  });
});

describe("completeSignIn", () => {
  function started(state = "s".repeat(32), returnTo = "/songs"): string {
    sessionStorage.setItem("mv:oauth-state", state);
    sessionStorage.setItem("mv:oauth-return", returnTo);
    return state;
  }

  it("refuses a callback this tab did not start, before any request", async () => {
    const session = useSessionStore();
    await expect(session.completeSignIn("code", "s".repeat(32))).rejects.toEqual(
      new SignInError("state"),
    );
    expect(calls).toEqual([]);
  });

  it("refuses a mismatched state, and clears the stored one", async () => {
    started("a".repeat(32));
    const session = useSessionStore();
    await expect(session.completeSignIn("code", "b".repeat(32))).rejects.toBeInstanceOf(
      SignInError,
    );
    expect(sessionStorage.getItem("mv:oauth-state")).toBeNull();
    expect(calls).toEqual([]);
  });

  it("signs in and lands where sign-in began", async () => {
    const state = started();
    route("/auth/github/callback", {
      status: 200,
      body: { token: "jwt", refreshToken: "rt-1", user: USER },
    });
    const session = useSessionStore();
    await expect(session.completeSignIn("code", state)).resolves.toBe("/songs");
    expect(session.user).toEqual(USER);
    expect(localStorage.getItem("mv:refresh-token")).toBe("rt-1");
  });

  it.each([
    ["the callback itself", "/auth/callback?code=old&state=old"],
    ["another origin", "//evil.example/x"],
    ["not a path", "https://evil.example"],
  ])("never lands on %s", async (_, returnTo) => {
    const state = started("s".repeat(32), returnTo);
    route("/auth/github/callback", {
      status: 200,
      body: { token: "jwt", refreshToken: "rt-1", user: USER },
    });
    const session = useSessionStore();
    await expect(session.completeSignIn("code", state)).resolves.toBe("/");
  });

  it("reports a code GitHub refused", async () => {
    const state = started();
    route("/auth/github/callback", { status: 400, body: { error: "bad_request", message: "x" } });
    const session = useSessionStore();
    await expect(session.completeSignIn("code", state)).rejects.toEqual(
      new SignInError("refused"),
    );
  });
});

describe("signIn", () => {
  it("leaves with a state the API accepts and keeps it for the callback", () => {
    const assign = vi.fn();
    vi.stubGlobal("location", { ...window.location, assign });
    useSessionStore().signIn("/songs");
    const url = new URL(assign.mock.calls[0][0]);
    const state = url.searchParams.get("state")!;
    expect(url.pathname).toBe("/api/auth/github/login");
    // contract §4: 16–128 characters of A–Z a–z 0–9 - . _ ~
    expect(state).toMatch(/^[A-Za-z0-9\-._~]{16,128}$/);
    expect(sessionStorage.getItem("mv:oauth-state")).toBe(state);
    expect(sessionStorage.getItem("mv:oauth-return")).toBe("/songs");
  });
});
