# Path Analysis of Track Geometry Sequences / 선로 기하 시퀀스 경로 분석

열차가 경험할 수 있는 **track geometry segments / 선로 기하 세그먼트** 의 모든 시퀀스를 포괄하는 **paths / 경로** 에 대한 설명이다.

이 분석은 **segment‑by‑segment control / 세그먼트 단위 제어** 나 고정된 개수의 연속 세그먼트 제어가 불가능하다는 가정에 기반한다. 그 이유는 선로 기하 세그먼트의 시퀀스가 **switch positions / 선로전환기 위치**—즉, 열차가 선택하는 **path / 경로**—에 따라 달라지기 때문이다 (**path dependent / 경로 의존적**).

**Brute force / 무차별 탐색** 방법은 인프라에 존재하는 모든 가능한 경로를 검사하는 것이다. 이는 최악의 경우, asymptotic 실행 시간이 **b\*2^n / b\*2^n** 이 되는데, 여기서 **b / b** 는 **model boundaries / 모델 경계** 의 수, **n / n** 은 동일한 주행 중 동시에 **against‑direction switches / 반대 방향 전환기** 로 설정될 수 있는 최대 선로전환기 개수다.

일반적인 인프라에서는 이러한 경로 대부분이 서로 많이 **overlap / 중첩** 되므로, 선로 기하 제어를 위해 수행되는 작업의 상당 부분이 불필요하다. 우리는 경로 의존성을 최대 길이 **l / l** 로 제한하여, 열차가 이 길이만큼 주행한 뒤에는 이후의 경로 선택이 더 이상 선로 기하 제어에 영향을 주지 않도록 할 수 있다. 이렇게 경로 의존성을 길이 **l / l** 로 제한하면, 최악의 asymptotic 실행 시간은 **b\*n\*2^m / b\*n\*2^m** 가 된다. 여기서 **m / m** 은 동일한 주행 중 **반대 방향** 이 될 수 있고 **l 미터 이내** 에 위치한 선로전환기 수다. 전형적인 철도 설비에서는 **m / m** 이 2–4 정도이므로, 인프라 규모가 커져도 지수적 실행 시간의 영향을 피할 수 있다.

알고리즘은 단순하다: **depth‑first search / 깊이 우선 탐색** 을 수행하며 전체 경로를 저장하되, 경로가 두 갈래로 나뉘면 하나는 그대로 두고 다른 하나는 **tail / 꼬리** 부분(길이 **l / l**)만 유지한다. **visited set / 방문 집합** 에는 경로의 꼬리만 저장하고, 현재 경로의 꼬리가 visited 집합에 있으면 탐색을 종료한다.

**Pseudo‑kode / 의사 코드**:
```rust
type Edge = (Node,Node,double); // Two nodes and the length between them.
type Path = vector<Edge>; // List of edges

vector<Path> paths(Infrastructure inf, double path_length_equality_margin) {
   visited = new set;
   output = new vector;
   stack = new stack;
   for each boundary {
      add the first edge from boundary to stack;
   }
   while current_path = stack.pop() {
      if path_length of current_path >= path_length_equality_margin {
         tail = current_path shortened to path_length_equality_margin from the end;
         if visited contains tail {
            add current_path to output;
            continue;
         } else {
            insert tail in visited;
         }
      }

      // other_side is the opposite node in the double node graph
      current_node = opposite dgraph node from the last node if current_path;

      switch on edges from current_node {
         case Single(target, length): // non-branching edge
            current_path.push(edge from current_node to target, length)
            push current_path to stack;
         case Switch(left_edge, right_edge): // branching edge
            new_path = current_path
            current_path.push(edge from current_node to left_edge.target, left_edge.length);
            new_path.push(edge from current_node to right_edge.target, right_edge.length);
            push current_path and new_path to stack;
         default: // Either a dead-end or a model boundary
            add current_path to output;
      }
   }
}

```
