import { css } from "./styled-system/css";
import { Box, VStack } from "./styled-system/jsx";

function App() {
  return (
    <Box minH="100dvh" bg="slate.2" px="4" py={{ base: "10", md: "16" }}>
      <VStack
        maxW="2xl"
        mx="auto"
        gap="6"
        p={{ base: "6", md: "8" }}
        bg="slate.1"
        borderWidth="1px"
        borderColor="slate.6"
        borderRadius="xl"
        boxShadow="sm"
        alignItems="flex-start"
      >
        <Box
          as="h1"
          fontSize={{ base: "2xl", md: "4xl" }}
          fontWeight="semibold"
          color="slate.12"
          lineHeight="short"
        >
          Harness Hat
        </Box>
        <Box color="slate.11">
          Zero-trust container management for AI coding agent sessions.
        </Box>
        <Box
          as="code"
          class={css({
            px: "2.5",
            py: "1",
            rounded: "md",
            bg: "slate.3",
            color: "blue.11",
            borderWidth: "1px",
            borderColor: "slate.6",
            fontSize: "sm",
          })}
        >
          Start with: npm run dev
        </Box>
      </VStack>
    </Box>
  );
}

export default App;
