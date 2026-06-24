// Canonical strict TypeScript profile — the maximally-strict compilerOptions set.
// SOURCE: https://www.typescriptlang.org/tsconfig (fetched 2026-06-24).

/// A strict `tsconfig.json` body `kavach lint init` writes for a TS/JS project.
pub(crate) const TS_TSCONFIG: &str = r#"{
  "compilerOptions": {
    "strict": true,
    "noImplicitAny": true,
    "strictNullChecks": true,
    "strictFunctionTypes": true,
    "strictBindCallApply": true,
    "strictPropertyInitialization": true,
    "noImplicitThis": true,
    "alwaysStrict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true,
    "noUncheckedIndexedAccess": true,
    "noPropertyAccessFromIndexSignature": true,
    "noImplicitOverride": true,
    "exactOptionalPropertyTypes": true,
    "useUnknownInCatchVariables": true
  }
}
"#;
