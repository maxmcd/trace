import { Hono } from "https://deno.land/x/hono/mod.ts";
import { serveStatic } from "https://deno.land/x/hono/middleware.ts";

const app = new Hono();

// Serve static files (index.html)
app.use("/", serveStatic({ root: "./" }));

// Serve data from traces.json
app.get("/data", async (c) => {
  try {
    const data = await Deno.readTextFile("./traces3.json");
    return c.json(JSON.parse(data));
  } catch (error) {
    console.error("Error reading traces.json:", error);
    return c.json({ error: "Failed to read data" }, 500);
  }
});

// Start the server
Deno.serve(app.fetch);
