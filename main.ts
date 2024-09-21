import { Hono } from "https://deno.land/x/hono/mod.ts";
import { AsyncLocalStorage } from "node:async_hooks";

// Minimal tracing library using AsyncLocalStorage
const traceContext = new AsyncLocalStorage<Span>();

type Span = {
  spanName: string;
  traceID: string;
  spanID: string;
  parentSpanID: string | null;
};

function generateSpanID(): string {
  return crypto.randomUUID();
}

type SpanLog = {
  event: "start" | "end";
  span: Span;
};

function logSpanStart(span: Span) {
  const log: SpanLog = {
    event: "start",
    span,
  };
  console.log(JSON.stringify(log));
}
function logSpanEnd(span: Span) {
  const log: SpanLog = {
    event: "end",
    span,
  };
  console.log(JSON.stringify(log));
}

async function newSpan<T>(name: string, fn: () => Promise<T> | T): Promise<T> {
  const parentContext = traceContext.getStore();
  const spanID = generateSpanID();
  const newContext = {
    spanName: name,
    spanID,
    parentSpanID: parentContext?.spanID ?? null,
    traceID: parentContext?.traceID ?? generateSpanID(),
  };
  logSpanStart(newContext);
  return traceContext.run(newContext, async () => {
    try {
      return await fn();
    } finally {
      logSpanEnd(newContext);
    }
  });
}

const app = new Hono();

// Helper function to sleep for a random duration
function randomSleep(max: number): Promise<void> {
  const duration = Math.floor(Math.random() * max);
  return new Promise((resolve) => setTimeout(resolve, duration));
}

// Function that may spawn child functions
async function complexOperation(): Promise<string> {
  await randomSleep(50);
  if (Math.random() > 0.5) {
    await newSpan("child-operation", async () => {
      await randomSleep(30);
    });
  }
  return "Operation complete";
}

// Root endpoint with nested spans
app.get("/", async (c) => {
  return newSpan("root-operation", async () => {
    const results = await Promise.all([
      newSpan("operation-1", async () => {
        await randomSleep(100);
        return complexOperation();
      }),
      newSpan("operation-2", async () => {
        await randomSleep(80);
        if (Math.random() > 0.7) {
          await newSpan("nested-operation", complexOperation);
        }
        return "Operation 2 complete";
      }),
      newSpan("operation-3", complexOperation),
    ]);
    return c.json({ message: "Root operation complete", results });
  });
});

// Serve data from traces.json
app.get("/data", async (c) => {
  return newSpan("fetch-data", async () => {
    try {
      const data = await Deno.readTextFile("./traces3.json");
      return c.json(JSON.parse(data));
    } catch (error) {
      console.error("Error reading traces.json:", error);
      return c.json({ error: "Failed to read data" }, 500);
    }
  });
});

// Start the server
Deno.serve({ port: 8080 }, (req) => {
  return newSpan(`${req.method} ${req.url}`, () => {
    return app.fetch(req);
  });
});
