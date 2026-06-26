import type { Rule } from "./Rule";
export type Rules = {
    rules: Array<Rule>;
    total?: number;
    page?: number;
    page_size?: number;
};
