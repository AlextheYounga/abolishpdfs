# Abstractions

An abstraction is valuable when it makes the code easier to understand.

Do not treat abstraction as unnecessary complexity by default. A named class, module, trait, interface, or value object can reduce complexity by giving behavior a clear name, responsibility, boundary, and home.

The goal is not to minimize the number of classes or files. The goal is to make the system read as a vocabulary of its domain rather than a sequence of implementation details.

## When to Create an Abstraction

Create an abstraction when it does one or more of the following:

* names a meaningful domain or application concept;
* gives a behavior one clear place to live;
* separates responsibilities or architectural boundaries;
* hides implementation details from callers;
* makes calling code shorter and easier to read;
* provides a reusable blueprint for an emerging pattern;
* keeps behavior consistent across multiple uses;
* creates a useful test seam;
* prevents a file, function, or entry point from accumulating mixed responsibilities.

An abstraction can be worthwhile with only one current caller. Repetition is not the only justification. Clarity, naming, ownership, and boundary placement are also valid reasons.

## Prefer Named Responsibilities

When a block of behavior represents a meaningful action or concept, consider giving it a named class, struct, module, or function.

Prefer:

```text
CreateUser.execute(command)
GenerateInvoice.execute(order)
DeploymentPlan.build(config)
AccessPolicy.authorize(user, resource)
```

over code that requires the reader to interpret a long sequence of unrelated operations at the call site.

The caller should communicate intent. The abstraction should contain the implementation details.

## Choose the Correct Boundary

Before creating code, ask:

1. What concept does this behavior represent?
2. Which layer owns that concept?
3. What information should enter the boundary?
4. What result or effect should leave it?
5. Which implementation details should callers not need to know?
6. Is this behavior part of an existing abstraction, or does it deserve a new one?

Do not place behavior wherever it is easiest to reach. Put it where its responsibility naturally belongs.

Entry points such as controllers, commands, routes, handlers, and UI components should usually coordinate work rather than contain substantial business behavior.

## Classes and Modules

Use a class, struct, or focused module when behavior benefits from:

* a meaningful name;
* explicit dependencies;
* internal state or configuration;
* multiple related operations;
* a public entry point with private supporting behavior;
* independent testing;
* reuse as a blueprint.

A class does not need inheritance, multiple implementations, or several callers to be legitimate.

Prefer focused objects with obvious responsibilities over large service containers, utility collections, or files containing unrelated functions.

## Reusable Blueprints

Reusable blueprints are encouraged when a pattern is real or clearly emerging.

A useful blueprint defines:

* the stable workflow or responsibility;
* the inputs and outputs;
* the extension points that genuinely vary;
* the constraints every implementation must follow.

Examples include:

* importers with a shared import process;
* deployment templates with common lifecycle steps;
* commands with consistent validation and execution;
* serializers with a shared output contract;
* policies with a common authorization interface;
* adapters around external systems;
* repositories around persistence boundaries.

Do not wait for an arbitrary number of duplicated implementations when the shared concept and boundary are already clear.

However, generalize only as far as current requirements justify. Do not add configuration, extension points, type parameters, hooks, or interfaces for imagined future cases.

## Traits, Interfaces, and Contracts

Use a trait, interface, or protocol when there is genuine variation or a meaningful architectural boundary.

Good reasons include:

* multiple implementations already exist or are expected by the current design;
* an external system needs to be replaceable;
* callers should depend on behavior rather than a concrete implementation;
* a shared workflow requires specific extension points;
* runtime or compile-time substitution is necessary;
* tests benefit from replacing an external boundary.

Do not create an interface merely because a class exists. A concrete class with a clear API is often sufficient.

Avoid empty contracts that simply repeat every public method of one implementation without providing meaningful substitution or boundary value.

## Composition Over Procedural Assembly

Prefer assembling focused objects and modules over repeatedly rebuilding workflows from loose functions, conditionals, and data structures.

A good abstraction should allow callers to say what they want done without reproducing how it is done.

Dependencies should be explicit through constructors, parameters, fields, or the native dependency mechanism of the language or framework.

Avoid hidden global dependencies, service location from arbitrary code, and behavior triggered through surprising side effects.

## Value Objects

Use value objects when a primitive value has domain meaning, constraints, or behavior.

Examples include:

```text
EmailAddress
ProjectName
Money
ReleaseId
DateRange
ServerAddress
```

A value object can:

* validate itself at construction;
* prevent invalid states;
* centralize formatting and comparison;
* make function signatures more expressive;
* prevent unrelated values with the same primitive type from being mixed up.

Do not leave important domain concepts represented indefinitely as ambiguous strings, integers, arrays, or maps when a small named type would make the code clearer.

## Avoid False Abstractions

An abstraction is harmful when it:

* has no meaningful name or responsibility;
* merely forwards arguments without clarifying the design;
* hides simple behavior behind several layers;
* combines unrelated concepts to appear reusable;
* exposes more configuration than callers need;
* depends on hypothetical future requirements;
* makes navigation harder than the code it replaces;
* requires understanding the abstraction before understanding the underlying problem;
* uses vague names such as `Manager`, `Helper`, `Util`, `Processor`, or `Base` without a precise domain meaning.

Do not reject an abstraction simply because it wraps something. Wrapping is useful when it creates a domain boundary, enforces constraints, simplifies callers, or isolates an external dependency.

Judge the abstraction by whether it improves understanding.

## Prefer Native Forms

Place substantial code in its native language and file type.

Do not embed long scripts, SQL, templates, configuration, or source code inside strings when they can live in dedicated files or structured APIs.

For example:

* shell logic belongs in a shell script;
* SQL belongs in a query file, query builder, or persistence module;
* templates belong in template files;
* configuration belongs in structured configuration;
* generated source should use a proper generator or template.

If quoting, escaping, or interpolation becomes difficult to reason about, the content is probably crossing the wrong boundary.

## File and Responsibility Boundaries

Prefer several focused files over one large file containing mixed responsibilities.

Files must not exceed 400 lines. At approximately 200 lines, reconsider whether the file contains multiple concepts or responsibilities.

Do not split files mechanically. Split when doing so gives a concept a clearer home, improves navigation, or separates behavior that changes for different reasons.

Each file should have an obvious purpose that can be described in one short sentence.

## Testing Abstractions

Test abstractions through their public behavior.

A useful abstraction should usually be testable without exercising unrelated parts of the system.

Use fixtures and factories when they make scenarios clearer and more representative. Avoid fixtures filled with irrelevant data.

Do not test private methods directly. Test the behavior that those methods support.

Interfaces and mocks should not be created only for tests when a simpler real implementation, fake, fixture, or in-memory boundary would be clearer.

## Refactoring Toward Abstractions

When adding behavior to existing code:

1. Identify the responsibility being added.
2. Decide whether the current file or object owns that responsibility.
3. Look for an existing concept or blueprint that should contain it.
4. Create a new named abstraction when the behavior deserves its own home.
5. Keep the public API small and intention-revealing.
6. Move implementation details behind the boundary.
7. Update callers so they express the workflow clearly.
8. Remove obsolete code and duplication exposed by the refactor.
9. Leave a runnable test for non-trivial behavior.

Do not perform broad architectural rewrites unrelated to the requested change. A small local refactor is appropriate when it is necessary to place the new behavior correctly.

## Final Standard

A good abstraction should make at least one of these questions easier to answer:

* What does this code do?
* Why does this behavior exist?
* Where should related behavior go?
* What can change independently?
* What assumptions are being enforced?
* How can this behavior be tested?
* What details should callers not need to understand?

Prefer abstractions that make the code read like a clear description of the system.

Do not abstract merely to reduce duplication.

Do abstract to improve meaning, ownership, boundaries, consistency, and readability.
