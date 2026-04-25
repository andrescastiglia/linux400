# Copilot Instructions (INSTRUCCIONES DE REFERENCIA)

Nota rápida: el contexto canónico y las decisiones de diseño o cambios deben basarse exclusivamente en estos cuatro documentos del repositorio:

- docs/KERNEL.md
- docs/PROJECT.md
- docs/cheetsheet.md
- docs/plan/implementation_plan.md

Propósito

Este archivo orienta a quienes automatizan cambios (Copilot, bots, revisores) sobre qué material tomar como fuente de verdad. Cualquier propuesta de cambio de comportamiento, requisito de plataforma, comandos o roadmap debe citar y alinearse con uno o más de los documentos listados arriba.

Guía práctica

- Referenciar el documento más relevante al proponer cambios:
  - implementation_plan.md → roadmap, criterios de aceptación y fases de trabajo.
  - PROJECT.md → arquitectura, componentes y modelo de objetos.
  - KERNEL.md → requisitos y limitaciones del kernel/plataforma (BPF, BTF, cgroups, xattrs, ZFS).
  - cheetsheet.md → semántica y forma esperada de comandos/CLI/TUI.

- Antes de modificar código: buscar y citar la sección exacta del/los documento(s) que justifican el cambio.
- Al documentar cambios: actualizar el/los documentos pertinentes si el cambio afecta su contenido o supuestos.
- Tests y verificaciones: seguir los criterios de aceptación en implementation_plan.md y validar mediante los scripts/tests ya presentes.

Convenciones adicionales

- Priorizar la persistencia `/l400`, el bootstrap y el roadmap de implementation_plan.md para decisiones de alto impacto.
- Para cambios de kernel o políticas de seguridad, cumplir las tablas y checklist de KERNEL.md.
- Para comandos y UX en TUI, respetar los nombres/firmas de cheetsheet.md.

Resumen

Estos cuatro archivos son la única fuente de verdad para contexto y cambios; cualquier excepción debe documentarse y justificarse explícitamente en la rama de la propuesta o en el PR.
