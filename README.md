# task scheduling algorithm

## requirements

- tasks with dependencies.
- deadlines in scheduling.
- task allocation based on priority, deadlines and task completion velocity
- non-working intervals.

## design overview

this scheduling algorithm generates an optimal timetable by iteratively allocating tasks onto a schedule based on their
heuristics score.

### each task includes:

- description: textual description.
- deadline: when the task should ideally be completed.
- granularity: minimum amount of continuous time allocated.

### allocators:

determine available intervals for tasks based on constraints.

- example: idleintervalallocator avoids scheduling tasks during predefined non-working intervals.

### heuristics:

guide the selection of the next task to schedule. multiple heuristics combine multiplicatively.

- priority heuristic: tasks with higher priority values are scheduled earlier.
- deadline-over-velocity heuristic: considers task deadline relative to complexity or expected duration.
- dependency heuristic: prevents scheduling tasks before their dependencies are satisfied.

### algorithm workflow:

1. task selection: iteratively selects the next task to schedule based on the combined heuristic score.
2. allocation: allocates selected tasks to appropriate intervals determined by the allocator.
3. iteration: repeats the process until all tasks are scheduled or no suitable intervals remain.
