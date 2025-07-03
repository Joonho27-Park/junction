# Sparse least squares / **희소 최소제곱**

This is a port from C code from [web.stanford.edu/group/SOL/software/lsqr](https://web.stanford.edu/group/SOL/software/lsqr/). It is a conjugate-gradient method for solving sparse linear equations and sparse least-squares problems. It solves `Ax = b`, or minimizes `|Ax-b|^2`, or minimizes the damped form `|Ax-b|^2 + l^2*|x|^2`. See the original web page for more details.

**이 프로젝트는** [Stanford SOL 그룹의 C 코드](https://web.stanford.edu/group/SOL/software/lsqr/) **를 Rust로 옮긴 것**입니다. 희소 선형 방정식 및 희소 최소제곱 문제를 푸는 **공액 그래디언트(conjugate-gradient) 방식**을 사용합니다. 즉 `Ax = b`를 풀거나 `|Ax-b|^2`을 최소화하며, 감쇠(damped) 형태인 `|Ax-b|^2 + l^2*|x|^2`도 처리할 수 있습니다. 자세한 내용은 원문을 참고해 주세요.

---

## TODO

* [ ] Take RHS array by move, as it is has unspecified contents afterwards.
  **우변(RHS) 배열을 이동(move)으로 받아, 이후 내용이 정의되지 않음을 명시하기.**
* [ ] Take an initial value as parameter.
  **초기 값을 파라미터로 받을 수 있도록 하기.**

---

## Usage / **사용법**

The Rust API is single function `lsqr` which takes the size of the matrix,
an initial suggestion for the solution vector, and a function that
the solver can call to update `y = y + A * x` or `x = x + A^T * y`,
leaving the representation of `A` up to the caller.
**Rust API는** `lsqr` **라는 단일 함수**입니다. 이 함수는

1. **행렬 크기**,
2. **초기 해 벡터 추정치**,
3. **곱셈을 수행할 콜백 함수**― `(y = y + A * x)` 또는 `(x = x + A^T * y)` ―를 받습니다.
   행렬 **`A`의 실제 저장 방식은 호출자에게 맡깁니다.**

Here is an example demonstrating the principle of calculating the required expressions,
but note that this is not actually a sparse representation, and in this case you might
be better off with a dense solver such as LAPACK (see [netlib LLS](https://www.netlib.org/lapack/lug/node27.html)).

**아래 예시는** 필요한 연산 방식을 보여 주기 위한 **간단한 데모**입니다. 실제로는 희소 행렬이 아니므로, 이런 경우라면 LAPACK 같은 **밀집(dense) 솔버**가 더 적합할 수 있습니다(참고: [netlib LLS](https://www.netlib.org/lapack/lug/node27.html)).

```rust
let params = Params {
    damp :0.0,         // Damping factor -- for miniminizing |Ax-b|^2 + damp^2 * x^2.
    rel_mat_err :1e-6, // Estimated relative error in the data defining the matrix A.
    rel_rhs_err :1e-6, // Estimated relative error in the right-hand side vector b.
    condlim :0.0,      // Upper limit on the condition number of A_bar (see original source code).
    iterlim :100,      // Limit on number of iterations
};

let mut rhs = vec![-1.,7.,2.];
let matrix = vec![3.,4.,0.,-6.,-8.,1.];
let n_rows = 3; let n_cols = 2;

let aprod = |mode :Product| {
    match mode {
	Product::YAddAx { x, y } =>  {
	    // y += A*x   [m*1] = [m*n]*[n*1]
	    for i in 0..n_rows {
		for j in 0..n_cols {
		    y[i] += matrix[n_rows*j + i] * x[j];
		}
	    }
	},
	Product::XAddATy { x, y } => {
	    // x += A^T*y  [n*1] = [n*m][m*1]
	    for i in 0..n_cols {
		for j in 0..n_rows {
		    x[i] += matrix[n_rows*i + j] * y[j];
		}
	    }
	},
    };
};

let (sol,statistics) = lsqr(|msg| print!("{}", msg), 
                            n_rows, n_cols, params, aprod, &mut rhs);
```