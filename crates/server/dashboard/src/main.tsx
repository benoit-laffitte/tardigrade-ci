import { ApolloClient, ApolloProvider, HttpLink, InMemoryCache } from "@apollo/client";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./styles.css";

// Central GraphQL client shared by the dashboard application.
const client = new ApolloClient({
  link: new HttpLink({ uri: "/graphql" }),
  cache: new InMemoryCache()
});

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("Dashboard root element not found");
}

createRoot(rootElement).render(
  <StrictMode>
    <ApolloProvider client={client}>
      <App />
    </ApolloProvider>
  </StrictMode>
);
