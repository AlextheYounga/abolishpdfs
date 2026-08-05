# Wise Clean Code

## Scope

Use this skill to decide:

- whether code needs to be written;
- where behavior belongs;
- when a class, module, trait, interface, or value object improves the design;
- how to separate business behavior from infrastructure;
- how to keep entry points thin;
- how to structure tests and fixtures;
- how to refactor without expanding the task unnecessarily;
- whether a design pattern solves a real problem;
- whether code is readable, coherent, and correctly bounded.

Do not use this skill to override the project's formatter, linter, framework
conventions, or explicit requirements.

## Desired Code Shape

Prefer code with:

- precise domain names;
- explicit control flow;
- focused functions, classes, modules, and files;
- small public APIs;
- visible dependencies;
- conventional locations;
- named responsibilities;
- behavior close to the concept that owns it;
- tests that describe behavior;
- abstractions that make callers easier to read;
- enough structure to reveal bad thinking early.

Do not optimize for the fewest lines, classes, or files.

Files must never exceed 400 lines. At approximately 200 lines, reconsider whether
the file contains multiple concepts or reasons to change.

## Naming

Use one consistent term for each concept throughout the codebase.

Prefer domain language over implementation language.

Avoid names such as `data`, `info`, `manager`, `helper`, `processor`, `thing`, or
`utils` when a more precise responsibility can be named.

### Avoid

```typescript
class DataManager {
  processInfo(data: unknown) {
    // ...
  }
}
```

### Prefer

```typescript
class InvoiceImporter {
  import(source: InvoiceFile): ImportResult {
    // ...
  }
}
```

The second version tells the reader what concept exists, what action it performs,
and what enters and leaves the boundary.

## Functions and Control Flow

A function should perform one coherent operation at one level of abstraction.

Prefer guard clauses, precise intermediate names, and high-level orchestration
over nested implementation detail.

### Avoid

```typescript
function publish(order: Order) {
  if (order.customer) {
    if (order.items.length > 0) {
      if (!order.cancelledAt) {
        const total = order.items.reduce((sum, item) => {
          return sum + item.price * item.quantity;
        }, 0);

        queue.send(JSON.stringify({
          type: "order.published",
          customer_id: order.customer.id,
          total,
        }));
      }
    }
  }
}
```

### Prefer

```typescript
function publishOrder(order: Order): void {
  ensurePublishable(order);

  const event = OrderPublished.from(order);
  eventBus.publish(event);
}
```

Supporting details now have names and homes. The entry point communicates the
workflow instead of forcing the reader to reconstruct it.

## Classes, Structs, and Modules

Create a named class, struct, module, or focused function when it:

- gives a meaningful concept a home;
- owns a coherent behavior;
- establishes a boundary;
- hides details callers should not know;
- makes orchestration easier to read;
- provides a reusable blueprint;
- creates a useful test seam;
- keeps a file or entry point from accumulating mixed responsibilities.

A class is justified when it improves understanding, even with one current caller.
Repetition is not the only reason to create an abstraction.

### Avoid

```python
def deploy(config):
    validate_server(config)
    release = create_release_directory(config)
    upload_files(config, release)
    install_dependencies(config, release)
    link_shared_files(config, release)
    switch_current_symlink(config, release)
    prune_old_releases(config)
```

This may be acceptable while tiny, but as deployment rules grow the function
becomes the accidental home of an entire concept.

### Prefer

```python
class Deployment:
    def __init__(
        self,
        releases: ReleaseStore,
        uploader: ArtifactUploader,
        dependencies: DependencyInstaller,
    ) -> None:
        self.releases = releases
        self.uploader = uploader
        self.dependencies = dependencies

    def execute(self, plan: DeploymentPlan) -> Release:
        release = self.releases.create(plan)
        self.uploader.upload(plan.artifact, release)
        self.dependencies.install(release)
        release.activate()
        self.releases.prune(plan.releases_to_keep)
        return release
```

`Deployment` is not needless ceremony. It names a substantial responsibility,
makes its dependencies visible, and provides a natural place for deployment
behavior to evolve.

## Reusable Blueprints

Reusable blueprints are encouraged when a concept or workflow is real or clearly
emerging.

A useful blueprint defines:

- what remains stable;
- which inputs and outputs cross the boundary;
- which steps or policies genuinely vary;
- which invariants every implementation must preserve;
- what callers should not need to know.

Do not wait for an arbitrary third duplication when the shared concept is already
clear. Do not generalize beyond current requirements and known callers.

### Good blueprint

```rust
pub trait ImportBlueprint {
    type Input;
    type Record;
    type Error;

    fn parse(&self, input: Self::Input) -> Result<Self::Record, Self::Error>;
    fn validate(&self, record: &Self::Record) -> Result<(), Self::Error>;
    fn persist(&self, record: Self::Record) -> Result<(), Self::Error>;

    fn execute(&self, input: Self::Input) -> Result<(), Self::Error> {
        let record = self.parse(input)?;
        self.validate(&record)?;
        self.persist(record)
    }
}
```

This trait is useful when multiple importers share a stable lifecycle and vary at
meaningful steps.

### Avoid speculative flexibility

```rust
pub trait UniversalProcessor<
    Input,
    Parsed,
    Validated,
    Persisted,
    Context,
    Options,
    Hooks,
> {
    // Twenty extension points for cases that do not exist.
}
```

The problem is not that it is generic. The problem is that its flexibility is not
grounded in known requirements.

## Interfaces, Traits, and Contracts

Use a trait, interface, protocol, or abstract contract when it expresses:

- genuine implementation variation;
- a meaningful architectural boundary;
- a shared workflow;
- runtime or compile-time substitution;
- isolation from an external system;
- a stable policy that should not depend on a concrete detail.

One implementation is acceptable when the boundary has present value.

Do not create an interface only because every class is expected to have one.

### Useful single-implementation boundary

```rust
pub trait PaymentGateway {
    fn charge(&self, payment: Payment) -> Result<Receipt, PaymentError>;
}
```

Even with one production gateway, this contract may protect application policy
from a vendor API and provide a clean fake for tests.

### Avoid hollow mirroring

```java
interface UserService {
    User getUser(long id);
}

class UserServiceImpl implements UserService {
    private final UserServiceDelegate delegate;

    public User getUser(long id) {
        return delegate.getUser(id);
    }
}
```

This structure adds no present responsibility, policy, translation, or boundary.
The issue is not delegation itself; it is delegation without a meaningful role.

## Value Objects

Use a value object when a primitive has domain meaning, constraints, invariants,
formatting, comparison rules, or behavior.

### Avoid

```typescript
function inviteUser(email: string, communityId: string): void {
  // Any string can cross the boundary.
}
```

### Prefer

```typescript
class EmailAddress {
  private constructor(readonly value: string) {}

  static parse(value: string): EmailAddress {
    if (!value.includes("@")) {
      throw new InvalidEmailAddress(value);
    }

    return new EmailAddress(value);
  }
}

class CommunityId {
  constructor(readonly value: string) {
    if (value.length === 0) {
      throw new InvalidCommunityId();
    }
  }
}

function inviteUser(
  email: EmailAddress,
  communityId: CommunityId,
): void {
  // Invalid values cannot reach this point.
}
```

Do not wrap every primitive mechanically. Wrap primitives when the name and rules
improve correctness or understanding.

## Entry Points and Orchestration

Controllers, routes, commands, handlers, CLI entry points, and UI actions should
usually translate input, validate, authorize, delegate, and return results.

They should not become the permanent home of business decisions.

### Avoid

```php
public function store(Request $request)
{
    $data = $request->validate([
        'email' => ['required', 'email'],
    ]);

    if (User::where('email', $data['email'])->exists()) {
        return back()->withErrors(['email' => 'Already registered']);
    }

    $user = User::create([
        'email' => $data['email'],
        'status' => 'pending',
    ]);

    Mail::to($user->email)->send(new WelcomeMail($user));

    return redirect()->route('users.show', $user);
}
```

### Prefer

```php
public function store(CreateUserRequest $request, CreateUser $createUser)
{
    $user = $createUser->execute($request->command());

    return redirect()->route('users.show', $user);
}
```

The action owns the use case. The request owns input validation and translation.
The controller remains readable orchestration.

## Architecture and Dependency Direction

Separate concerns when they have different responsibilities or reasons to change.

Core business policy should not depend directly on HTTP requests, ORM models,
database drivers, UI frameworks, or vendor SDKs when a meaningful boundary can
protect it.

Dependencies should generally point from external details toward stable policy.

### Avoid

```python
from flask import Request
from sqlalchemy.orm import Session

def create_user(request: Request, session: Session):
    user = UserModel(email=request.json["email"])
    session.add(user)
    session.commit()
    return user
```

Application behavior is coupled to both transport and persistence.

### Prefer

```python
@dataclass(frozen=True)
class CreateUserCommand:
    email: EmailAddress


class UserRepository(Protocol):
    def add(self, user: User) -> None: ...


class CreateUser:
    def __init__(self, users: UserRepository) -> None:
        self.users = users

    def execute(self, command: CreateUserCommand) -> User:
        user = User.register(command.email)
        self.users.add(user)
        return user
```

HTTP and database adapters translate into and out of this boundary.

Do not force ceremonial layers around trivial CRUD. Add boundaries where they
protect meaningful policy, enable independent change, or improve understanding.

## External Systems and Adapters

Wrap external systems when doing so:

- translates vendor terminology into domain terminology;
- isolates a volatile API;
- centralizes retries, authentication, serialization, or error mapping;
- prevents vendor types from spreading through the application;
- gives tests a stable boundary.

### Prefer

```typescript
class StripePaymentGateway implements PaymentGateway {
  constructor(private readonly stripe: Stripe) {}

  async charge(payment: Payment): Promise<Receipt> {
    try {
      const intent = await this.stripe.paymentIntents.create({
        amount: payment.amount.cents,
        currency: payment.amount.currency,
        payment_method: payment.methodId,
        confirm: true,
      });

      return StripeReceiptMapper.toDomain(intent);
    } catch (error) {
      throw StripeErrorMapper.toPaymentError(error);
    }
  }
}
```

This wrapper has a clear responsibility. It protects the rest of the system from
Stripe-specific types, terminology, and errors.

## Native Representation and String Wrangling

Do not embed substantial scripts, SQL, templates, configuration, or source code
inside strings.

Put substantial code in its native file type and invoke or load it through a
clear boundary.

### Avoid

```python
script = f"""
set -e
cd "{release_path}"
ln -sfn "{shared_path}/storage" storage
chown -R "{runtime_user}:{runtime_group}" storage
systemctl restart "{service_name}"
"""

subprocess.run(["bash", "-c", script], check=True)
```

This mixes Python, shell, interpolation, escaping, and deployment policy in one
place.

### Prefer

```python
subprocess.run(
    [
        scripts_dir / "activate-release.sh",
        release_path,
        shared_path,
        runtime_user,
        runtime_group,
        service_name,
    ],
    check=True,
)
```

```bash
#!/usr/bin/env bash
set -euo pipefail

release_path=$1
shared_path=$2
runtime_user=$3
runtime_group=$4
service_name=$5

cd "$release_path"
ln -sfn "$shared_path/storage" storage
chown -R "$runtime_user:$runtime_group" storage
systemctl restart "$service_name"
```

The shell script can now be linted, tested, executed independently, and read in
its native language.

Small, fixed commands passed as argument arrays are not string wrangling.

## Comments

Prefer code that explains itself through names and structure.

Comments should explain:

- why a decision exists;
- a non-obvious business rule;
- an external-system quirk;
- an intentional tradeoff;
- a safety constraint;
- why an apparently simpler solution is incorrect.

### Avoid

```python
# Increment retry count
retry_count += 1
```

### Prefer

```python
# The vendor may accept a request after timing out locally, so retries must keep
# the same idempotency key to avoid duplicate charges.
retry_count += 1
```

Do not use comments to compensate for code that can be made clear through naming
or extraction.

## Testing

Every non-trivial behavioral change must leave behind a runnable test.

Use the project's existing test framework and conventions.

Prefer tests that are:

- behavior-focused;
- descriptive;
- fast enough for their level;
- independent;
- repeatable;
- self-validating;
- clear about setup, action, and expected result.

Fixtures and factories are encouraged when they make scenarios clearer and more
representative.

### Avoid noisy inline setup

```python
def test_suspends_customer_after_failed_payments():
    customer = Customer(
        id="customer-123",
        email="alex@example.com",
        first_name="Alex",
        last_name="Younger",
        locale="en-US",
        timezone="America/Chicago",
        marketing_opt_in=False,
        failed_payments=3,
        status="active",
        created_at=datetime(2026, 1, 1),
        updated_at=datetime(2026, 1, 1),
    )

    customer.record_failed_payment()

    assert customer.status == "suspended"
```

### Prefer a focused fixture or factory

```python
def test_suspends_customer_after_fourth_failed_payment():
    customer = customer_fixture(
        failed_payments=3,
        status=CustomerStatus.ACTIVE,
    )

    customer.record_failed_payment()

    assert customer.status is CustomerStatus.SUSPENDED
```

The test emphasizes the behavior rather than irrelevant construction details.

Test observable behavior rather than private methods. A test that is difficult to
write is evidence that the design or boundary may be wrong.

When fixing a bug, add a regression test that reproduces it when practical.

Trivial declarations, framework wiring, and mechanical delegation do not require
dedicated tests unless they carry meaningful behavior.

## Refactoring

Refactor in small, coherent, behavior-preserving steps.

When adding behavior:

1. Identify the responsibility being added.
2. Decide whether the current location owns it.
3. Look for an existing concept or blueprint that should contain it.
4. Create a named abstraction when the behavior deserves a clearer home.
5. Keep the public API small.
6. Move supporting details behind the boundary.
7. Update callers so they express intent clearly.
8. Remove obsolete code and duplication exposed by the change.
9. Run relevant tests and checks.

Refactor nearby code when necessary to place behavior correctly. Avoid broad,
unrelated rewrites that obscure the requested change.

## Correctness

Never reduce quality merely to finish faster. Reduce scope before sacrificing
correctness.

Never cut corners on:

- validation at trust boundaries;
- correct terminology;
- authentication and authorization;
- security and privacy;
- accessibility;
- data integrity and transactional consistency;
- concurrency and synchronization;
- error handling that prevents data loss or corruption;
- realistic handling of clocks, networks, hardware, sensors, and external services;
- behavior explicitly required by the task.

When two approaches are similarly clear, choose the one that handles edge cases
correctly and fails predictably.

## Review Heuristics

Look for:

- **Rigidity:** a small change requires edits in many places.
- **Fragility:** a change breaks unrelated behavior.
- **Immobility:** useful behavior cannot be separated from its current context.
- **Viscosity:** shortcuts are easier than the intended design.
- **Needless complexity:** structure exists without a present responsibility.
- **Needless repetition:** the same decision is implemented inconsistently.
- **Opacity:** intent, ownership, or control flow is difficult to understand.
- **Boundary leakage:** framework, persistence, transport, or vendor details
  spread beyond their proper boundary.
- **Mixed responsibilities:** one function, class, or file changes for unrelated reasons.
- **Hidden dependencies:** behavior relies on globals, implicit state, or surprising side effects.
- **String wrangling:** substantial native code is trapped inside strings.
- **Untested behavior:** meaningful behavior changed without a runnable check.
- **Oversized files:** any file exceeds the 400-line hard limit.

When reviewing code:

1. Point to the exact file, class, or function.
2. Name the concrete problem.
3. Explain why it makes the code harder to understand, test, or change.
4. Suggest the smallest coherent refactor.
5. Distinguish required fixes from optional improvements.
6. Do not merely say that code is unclean or violates SOLID.

## Final Standard

Good code should make these questions easy to answer:

- What behavior does this provide?
- Why does it exist?
- Where does related behavior belong?
- What can change independently?
- Which assumptions and invariants are enforced?
- Which details are hidden from callers?
- How can the behavior be tested safely?

Prefer code that reveals the system's concepts, decisions, and boundaries at a
glance.
