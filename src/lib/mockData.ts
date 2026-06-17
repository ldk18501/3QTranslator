import type { Definition } from "./types";

export const fallbackDefinitions: Record<string, Definition[]> = {
  salt: [
    {
      partOfSpeech: "noun",
      meaning: "盐；食盐；用于调味或保存食物的晶体物质。",
      example: "Add a little salt before serving.",
      synonyms: ["seasoning", "sodium chloride"],
    },
    {
      partOfSpeech: "verb",
      meaning: "给食物加盐；用盐保存。",
      example: "They salted the fish for winter.",
    },
  ],
  learn: [
    {
      partOfSpeech: "verb",
      meaning: "学习；通过经验或教学获得知识。",
      example: "She wants to learn Japanese this year.",
    },
  ],
};
