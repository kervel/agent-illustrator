---
parent_branch: main
feature_number: "005"
status: Active (constraint solver now available on main)
created_at: 2026-01-23T00:00:00+00:00
---

# Feature: Reusable Components (SVG and AIL Imports)

## Overview

Enable users to create reusable visual components through three mechanisms:

1. **Inline template blocks** — Define reusable components directly in the same file
2. **AIL file imports** — Import components from external `.ail` files
3. **SVG file imports** — Import components from external `.svg` files

All three mechanisms use the same instantiation syntax, allowing a template defined inline to be used identically to one imported from a file. This promotes consistency, reduces duplication, and enables building complex illustrations from simpler building blocks.

The primary use case is defining a visual element once (e.g., a person icon as an SVG, a complex machine as an AIL file, or a simple labeled box as an inline template) and then using it multiple times in a composition.

## Clarifications

### Session 2026-01-23

- Q: How should component instance sizing work when the SVG has intrinsic dimensions? → A: Scale to fit layout (preserving aspect ratio) by default; allow optional explicit size override parameters.
- Q: Can connections target elements inside a component instance? → A: Yes, but only via explicitly exported connection points; components must declare which internal elements are targetable.
- Q: What happens if a component/instance name conflicts with existing names? → A: Scoped namespaces; component internals have their own namespace, preventing conflicts with parent document names.
- Q: Can parent constraints override component internal constraints? → A: No, encapsulation preserved. Parent can only constrain exported elements, not modify internal layout.

### Session 2026-01-24

- Q: Can inline templates reference other inline templates defined in the same file (nesting)? → A: Yes, order-independent; templates can reference any template in the same file regardless of declaration order (declarative, not procedural).
- Q: Should 'template' be the primary keyword with 'component' as alias? → A: No aliases; use single keyword only (simplicity over backward compatibility).
- Q: How should template parameters and connection points be declared? → A: AIL templates (inline or file): explicit parameter declarations and explicit export statements. SVG templates: no parameters; connection points limited to bounding box edges (like a rectangle).

## User Scenarios

### Scenario 1: Define and Use an Inline Template

A user wants to create a reusable "labeled box" component directly in their document without creating separate files.

**Acceptance Criteria:**
- User can define an inline template block with a name
- The template block contains standard AIL statements (shapes, connections, layouts)
- User can instantiate the template multiple times with different instance names
- Template instances can be styled or parameterized independently

**Example:**
```
template "labeled_box" {
    rect box
    text label [content: "Default"]
    row { box label }
    export box
}

labeled_box "input" [label.content: "Input"]
labeled_box "output" [label.content: "Output"]
connect input -> output
```

### Scenario 2: Import and Use an SVG Icon Multiple Times

A user has an SVG file representing a person icon. They want to create a team diagram showing 5 people arranged in a row.

**Acceptance Criteria:**
- User can declare an SVG file as a component with a name
- User can instantiate that component multiple times with different instance names
- Each instance renders the same SVG at the appropriate position
- Instances can be styled or modified independently (e.g., different colors)

### Scenario 2: Import and Use an AIL File as a Subcomponent

A user has an AIL file defining a "server rack" with multiple servers inside. They want to create a datacenter diagram with 3 server racks.

**Acceptance Criteria:**
- User can import an AIL file and give it a component name
- User can instantiate that AIL component multiple times
- The imported AIL's internal structure is preserved in each instance
- Connections can be made to elements inside the imported component

### Scenario 3: Component with Configurable Parameters

A user has a "labeled box" component that takes a text label as a parameter. They want to create multiple boxes with different labels.

**Acceptance Criteria:**
- Components can declare parameters (at minimum, a text label)
- When instantiating, users can provide parameter values
- The component renders with the provided parameter values

### Scenario 4: Nested Component Imports

A user creates a "workstation" AIL component that imports a "monitor" SVG and a "keyboard" SVG. Another file imports the "workstation" component.

**Acceptance Criteria:**
- Components can import other components (transitive imports)
- Import resolution handles nested dependencies correctly
- Circular import dependencies are detected and reported as errors

### Scenario 5: Connecting to Exported Component Ports

A user has a "router" component with exported "wan" and "lan" ports. They want to connect cables to specific ports rather than the router's bounding box.

**Acceptance Criteria:**
- Component can declare exports for internal elements
- External connections can target exports via dot notation (e.g., `router1.wan`)
- Connections to non-exported internals produce clear error messages
- Exported ports are visually indicated as valid connection targets

### Scenario 6: Missing Import Error Handling

A user references an import file that doesn't exist or has syntax errors.

**Acceptance Criteria:**
- Clear error message indicates which import file is missing
- Error message includes the line where the import was declared
- If the imported file has syntax errors, those errors are reported with context

## Functional Requirements

### FR-1: Template Declaration Syntax (Unified)

The grammar must support three ways to declare reusable templates:

**Requirement:** Users can define templates via inline blocks, external AIL files, or external SVG files. All three use the same instantiation syntax.

**Testable Criteria:**
- Parser accepts inline template blocks:
  ```
  template "person" {
      circle head
      rect body
  }
  ```
  and produces an AST node with template_name="person", source_type=Inline, containing parsed AIL statements
- Parser accepts `template "person" from "icons/person.svg"` and produces an AST node with template_name="person", source_path="icons/person.svg", source_type=SVG
- Parser accepts `template "rack" from "components/server-rack.ail"` and produces an AST node with source_type=AIL
- Single `template` keyword used (no aliases for simplicity)

### FR-2: Component Instantiation Syntax

The grammar must support creating instances of declared components.

**Requirement:** Users can create named instances of components.

**Testable Criteria:**
- Parser accepts `person "alice"` (after component declaration) and produces an instance AST node
- Parser accepts `rack "rack1"` and produces an instance AST node referencing the component

### FR-3: Template Parameters (AIL only)

AIL templates (inline or file) can accept explicit parameters for customization. SVG templates do not support parameters.

**Requirement:** AIL templates declare parameters explicitly; users provide values when instantiating.

**Testable Criteria:**
- Parser accepts explicit parameter declaration: `template "box" (label: "Default", color: blue) { ... }`
- Parser accepts `box "mybox" [label: "Custom"]` and captures parameters in AST
- Parameters are optional; defaults from declaration used when omitted
- SVG templates reject parameters (error: "SVG templates do not support parameters")

### FR-4: Template Resolution

The system must resolve all templates (inline and imported) before instantiation.

**Requirement:** Template resolution is order-independent within a file. Import paths are resolved relative to the importing file's directory. Circular dependencies are the only ordering constraint.

**Testable Criteria:**
- `template "x" from "sub/file.svg"` resolves to `{current_file_dir}/sub/file.svg`
- Missing files produce clear error messages with the resolved path
- Circular dependencies are detected and reported as errors (A uses B, B uses A)
- Forward references work: an instance can appear before its template declaration in the file

### FR-5: SVG Component Rendering

SVG components must be embedded in the output.

**Requirement:** SVG content is included in the rendered output, properly positioned and scaled to fit its layout allocation while preserving aspect ratio. Optional explicit size parameters override default scaling.

**Testable Criteria:**
- An SVG component instance appears at the layout-determined position
- The SVG scales to fit its allocated layout space while preserving aspect ratio
- Optional `width` and `height` parameters override automatic scaling: `person "alice" [width: 100, height: 150]`
- Multiple instances of the same SVG component render independently

### FR-6: AIL Component Rendering

AIL components must be rendered as nested structures.

**Requirement:** Imported AIL files are parsed and their elements become part of the parent document.

**Testable Criteria:**
- An AIL component's shapes appear in the rendered output
- Layout of the AIL component's contents is preserved
- The component acts as a group for layout purposes in the parent

### FR-7: Style Propagation

Styles can be applied to component instances.

**Requirement:** Styles applied to an instance affect its visual appearance.

**Testable Criteria:**
- `person "alice" [fill: blue]` overrides or supplements the SVG's default colors
- Style inheritance follows predictable rules (instance styles override component defaults)

### FR-8: Connection to Template Instances

Connections can target template instances and their exported connection points.

**Requirement:** Templates can participate in connections. AIL templates may export named connection points from internal elements. SVG templates only support bounding box connection (like rectangles).

**Testable Criteria:**
- `connect server1 -> rack1` works when rack1 is a template instance (attaches to bounding box)
- AIL templates can declare exports: `export port1, port2` within the template definition
- External connections can target AIL template exports: `connect cable -> rack1.port1`
- Attempting to connect to a non-exported internal element produces a clear error
- SVG template instances only support bounding box connections (no dot notation)

### FR-9: Template Export Declaration (AIL only)

AIL templates can expose internal elements as connection targets. SVG templates do not support exports.

**Requirement:** AIL templates (inline or file) declare which internal elements are accessible from outside using an export statement.

**Testable Criteria:**
- Parser accepts `export element_name` within an AIL template's body
- Multiple exports supported: `export input, output, status`
- Exported names must reference existing elements in the template (error if not found)
- Only exported elements are visible via dot notation from parent scope
- Export statements in SVG templates are not applicable (SVG content cannot declare exports)

### FR-10: Constraint Scoping for Imported AIL

**Dependency:** Requires constraint solver (Feature 005-constraint-solver)

Constraints in imported AIL files must integrate with the parent document's constraint system while maintaining encapsulation.

**Requirement:** Internal component constraints are solved within their namespace. Parent documents can reference exported elements in constraints but cannot override internal constraints.

**Testable Criteria:**
- Internal constraints (e.g., `constrain a.left = b.left` inside component) are solved as part of the global constraint system
- Parent can reference exported elements: `constrain cable.right = router1.wan.left`
- Parent constraints cannot reference non-exported internals (produces clear error: "element 'router1.internal_node' is not exported")
- Constraint conflicts between parent and component produce scoped error messages indicating which file contains each conflicting constraint
- Component internal layout is stable regardless of how parent positions the component instance

## Success Criteria

1. **Reuse Efficiency**: A user can define a complex visual element once and instantiate it 10+ times without duplicating code
2. **First-Attempt Correctness**: AI agents can correctly use component syntax 90% of the time by following documentation
3. **Parse Speed**: Documents with 50 component imports and 100 instances parse in under 200 milliseconds
4. **Error Clarity**: Missing import errors clearly indicate the file path and location in source
5. **Composition Depth**: Supports at least 5 levels of nested component imports without performance degradation

## Key Entities

### Template (formerly Component)

A reusable template defined from one of three sources:
1. **Inline block** — AIL statements defined directly in the document
2. **External AIL file** — Imported from a `.ail` file
3. **External SVG file** — Imported from a `.svg` file

Has a name, source type (Inline/AIL/SVG), optional source path (for file imports), content (inline statements or loaded file), and optional parameter definitions.

### ComponentInstance

A usage of a component with a specific instance name, optional parameter values, and optional style modifiers.

### ImportSource

Represents the external file being imported: path, type (SVG or AIL), and resolved content.

### ComponentParameter

A named parameter that can customize component behavior. Has a name, type (initially just string/text), and optional default value.

### ComponentExport

A declaration that makes an internal element accessible from outside the component. Enables dot-notation connections (e.g., `instance.exportedName`).

## Assumptions

1. **File System Access**: The renderer has read access to import paths relative to the source file
2. **SVG Subset**: SVGs are assumed to be static (no JavaScript, no external references)
3. **No Circular Dependencies**: Circular imports are an error, not a feature
4. **UTF-8 Files**: All imported files are UTF-8 encoded
5. **Flat Parameter Space**: Parameters are simple key-value pairs, not nested structures
6. **Scoped Namespaces**: Each component has its own internal namespace; internal element names do not conflict with parent document names. Component and instance names in the parent scope must be unique within that scope.
7. **Order-Independent Declarations**: Template declarations and instantiations can appear in any order within a file. The language is declarative; all templates are resolved before instantiation (forward references allowed).
8. **Aspect Ratio Preservation**: When scaling components to fit layout, aspect ratio is always preserved (no distortion)
9. **Constraint Solver Available**: This feature depends on the constraint-based layout system (Feature 005-constraint-solver). Imported AIL constraints merge into the global constraint system.

## Technical Boundaries

This feature covers:
- Grammar extensions for component declaration and instantiation
- AST types for components and instances
- Import resolution logic
- Error handling for missing/invalid imports
- Integration with existing layout system

This feature does NOT cover:
- Remote URL imports (only local files)
- Dynamic/runtime component loading
- Component versioning or package management
- Binary file imports (only text-based SVG and AIL)
- Animation within SVG components
