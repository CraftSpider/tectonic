use crate::c_api::core::scaled_t;
use crate::c_api::engine::{arith_error, help_line, help_ptr, tex_remainder};
use crate::c_api::errors::error;
use crate::c_api::output::{
    capture_to_diagnostic, error_here_with_diagnostic, print_scaled, print_str,
};
use std::cell::{Cell, RefCell};
use std::{mem, ptr};

pub const TWO_TO_THE: [i32; 31] = {
    let mut arr = [1; 31];
    let mut idx = 1;
    while idx < 31 {
        arr[idx] = 2 * arr[idx - 1];
        idx += 1;
    }
    arr
};

const SPEC_LOG: [i32; 29] = {
    let mut spec_log = [0; 29];
    spec_log[1] = 93032640;
    spec_log[2] = 38612034;
    spec_log[3] = 17922280;
    spec_log[4] = 8662214;
    spec_log[5] = 4261238;
    spec_log[6] = 2113709;
    spec_log[7] = 1052693;
    spec_log[8] = 525315;
    spec_log[9] = 262400;
    spec_log[10] = 131136;
    spec_log[11] = 65552;
    spec_log[12] = 32772;
    spec_log[13] = 16385;
    let mut idx = 14;
    while idx < 28 {
        spec_log[idx] = TWO_TO_THE[27 - idx];
        idx += 1;
    }
    spec_log[28] = 1;
    spec_log
};

thread_local! {
    static RANDOMS: RefCell<[i32; 55]> = const { RefCell::new([0; 55]) };
    static J_RANDOM: Cell<usize> = const { Cell::new(0) };
}

#[no_mangle]
pub extern "C" fn tex_round(r: f64) -> i32 {
    /* We must reproduce very particular rounding semantics to pass the TRIP
     * test. Specifically, values within the 32-bit range of TeX int32_ts are
     * rounded to the nearest int32_t with half-integral values going away
     * from zero: 0.5 => 1, -0.5 => -1.
     *
     * `r` does not necessarily lie within the range of a 32-bit TeX int32_t;
     * if it doesn't, we clip. The following LaTeX document allegedly triggers
     * that codepath:
     *
     *   \documentstyle{article}
     *   \begin{document}
     *   \begin{flushleft}
     *   $\hbox{} $\hfill
     *   \filbreak
     *   \eject
     *
     */

    if r > 2147483647.0 {
        return 0x7FFFFFFF;
    }
    if r < -2147483648.0 {
        return -0x80000000;
    }

    /* ANSI defines the float-to-int32_t cast to truncate towards zero, so the
     * following code is all that's necessary to get the desired behavior. The
     * truncation technically causes an uncaught "inexact" floating-point
     * exception, but exception is virtually impossible to avoid in real
     * code. */

    if r >= 0.0 {
        (r + 0.5) as i32
    } else {
        (r - 0.5) as i32
    }
}

#[no_mangle]
pub extern "C" fn half(x: i32) -> i32 {
    // TODO: This is just div_ceil for a fixed divisor of 2
    if x % 2 != 0 {
        (x + 1) / 2
    } else {
        x / 2
    }
}

#[no_mangle]
pub unsafe extern "C" fn mult_and_add(
    n: i32,
    x: scaled_t,
    y: scaled_t,
    max_answer: scaled_t,
) -> scaled_t {
    if n < 0 {
        mult_and_add(-n, -x, y, max_answer)
    } else if n == 0 {
        y
    } else if x <= (max_answer - y) / n && -x <= (max_answer + y) / n {
        n * x + y
    } else {
        *arith_error = true;
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn x_over_n(x: scaled_t, n: i32) -> scaled_t {
    if n < 0 {
        *tex_remainder = -*tex_remainder;
        x_over_n(-x, -n)
    } else if n == 0 {
        *arith_error = true;
        *tex_remainder = x;
        0
    } else if x >= 0 {
        *tex_remainder = x % n;
        x / n
    } else {
        *tex_remainder = -(-x % n);
        -(-x / n)
    }
}

#[no_mangle]
pub unsafe extern "C" fn xn_over_d(mut x: scaled_t, n: i32, d: i32) -> scaled_t {
    const MAGIC: i32 = 32768;

    let positive = if x >= 0 {
        true
    } else {
        x = -x;
        false
    };

    let t = x % MAGIC * n;
    let mut u = (x / MAGIC) * n + (t / MAGIC);
    let v = (u % d) * MAGIC + (t % MAGIC);

    if u / d >= MAGIC {
        *arith_error = true;
    } else {
        u = MAGIC * (u / d) + (v / d);
    }

    if positive {
        *tex_remainder = v % d;
        u
    } else {
        *tex_remainder = -(v % d);
        -u
    }
}

#[no_mangle]
pub unsafe extern "C" fn round_xn_over_d(mut x: scaled_t, n: i32, d: i32) -> scaled_t {
    const MAGIC: i32 = 0x8000;

    let positive = if x >= 0 {
        true
    } else {
        x = -x;
        false
    };

    let t = x % MAGIC * n;
    let mut u = (x / MAGIC) * n + (t / MAGIC);
    let mut v = (u % d) * MAGIC + (t % MAGIC);

    if u / d >= MAGIC {
        *arith_error = true;
    } else {
        u = MAGIC * (u / d) + (v / d);
    }

    v = v % d;
    if 2 * v >= d {
        u += 1;
    }
    if positive {
        u
    } else {
        -u
    }
}

unsafe fn make_frac(mut p: i32, mut q: i32) -> i32 {
    let mut negative = if p >= 0 {
        false
    } else {
        p = -p;
        true
    };

    if q <= 0 {
        q = -q;
        negative = !negative;
    }

    let n = p / q;
    let mut p = p % q;

    if n >= 8 {
        *arith_error = true;
        if negative {
            -0x7FFFFFFF
        } else {
            0x7FFFFFFF
        }
    } else {
        let n = (n - 1) * 0x10000000;
        let mut f = 1;

        loop {
            let be_careful = p - q;
            p = be_careful + p;
            if p >= 0 {
                f = 2 * f + 1;
            } else {
                f = 2 * f;
                p = p + q;
            }

            if f >= 0x10000000 {
                break;
            }
        }

        let be_careful = p - q;
        if be_careful + p >= 0 {
            f += 1;
        }

        if negative {
            -(f + n)
        } else {
            f + n
        }
    }
}

unsafe fn take_frac(mut q: i32, mut f: i32) -> i32 {
    let mut negative = if f >= 0 {
        false
    } else {
        f = -f;
        true
    };

    if q < 0 {
        q = -q;
        negative = !negative;
    }

    let mut n;
    if f < 0x10000000 {
        n = 0;
    } else {
        n = f / 0x10000000;
        f = f % 0x10000000;

        if q <= 0x7FFFFFFF / n {
            n = n * q;
        } else {
            *arith_error = true;
            n = 0x7FFFFFFF;
        }
    }

    f = f + 0x10000000;
    let mut p = 0x08000000;

    if q < 0x40000000 {
        loop {
            if f % 2 != 0 {
                p = (p + q) / 2;
            } else {
                p = p / 2;
            }
            f = f / 2;
            if f == 1 {
                break;
            }
        }
    } else {
        loop {
            if f % 2 != 0 {
                p = p + (q - p) / 2;
            } else {
                p = p / 2;
            }
            f = f / 2;
            if f == 1 {
                break;
            }
        }
    }

    let be_careful = n - 0x7FFFFFFF;
    if be_careful + p > 0 {
        *arith_error = true;
        n = 0x7FFFFFFF - p;
    }

    if negative {
        -(n + p)
    } else {
        n + p
    }
}

unsafe fn m_log(mut x: i32) -> i32 {
    if x <= 0 {
        error_here_with_diagnostic(c!("Logarithm of "));
        print_scaled(x);
        print_str(b" has been replaced by 0");
        capture_to_diagnostic(ptr::null_mut());
        *help_ptr = 2;
        let help_line_m = &mut *help_line;
        help_line_m[1] = c!("Since I don't take logs of non-positive numbers,");
        help_line_m[0] = c!("I'm zeroing this one. Proceed, with fingers crossed.");
        error();
        return 0;
    }

    let mut y = 1302456860;
    let mut z = 6581195;

    while x < 0x40000000 {
        x *= 2;
        y -= 93032639;
        z -= 48782;
    }

    y += z / 65536;
    let mut k = 2;

    while x > 0x40000004 {
        z = ((x - 1) / TWO_TO_THE[k]) + 1;

        while x < 0x40000000 + z {
            z = (z + 1) / 2;
            k += 1;
        }

        y += SPEC_LOG[k];
        x -= z;
    }

    y / 8
}

fn ab_vs_cd(mut a: i32, mut b: i32, mut c: i32, mut d: i32) -> i32 {
    if a < 0 {
        a = -a;
        b = -b;
    }

    if c < 0 {
        c = -c;
        d = -d;
    }

    if d <= 0 {
        if b >= 0 {
            return if (a == 0 || b == 0) && (c == 0 || d == 0) {
                0
            } else {
                1
            };
        }

        if d == 0 {
            return if a == 0 { 0 } else { -1 };
        }

        mem::swap(&mut a, &mut c);
        mem::swap(&mut b, &mut d);
        b = -b;
        d = -d;
    } else if b <= 0 {
        return if b < 0 && a > 0 {
            -1
        } else if c == 0 {
            0
        } else {
            -1
        };
    }

    loop {
        let q = a / d;
        let r = c / b;

        if q != r {
            return if q > r { 1 } else { -1 };
        }

        let q = a % d;
        let r = c % b;

        if r == 0 {
            return if q == 0 { 0 } else { 1 };
        }

        if q == 0 {
            return -1;
        }

        a = b;
        b = q;
        c = d;
        d = r;
    }
}

unsafe fn new_randoms(randoms: &mut [i32; 55]) {
    for k in 0..24 {
        let mut x = randoms[k] - randoms[k + 31];
        if x < 0 {
            x += 0x10000000;
        }
        randoms[k] = x;
    }

    for k in 24..55 {
        let mut x = randoms[k] - randoms[k - 24];
        if x < 0 {
            x += 0x10000000;
        }
        randoms[k] = x;
    }

    J_RANDOM.set(54);
}

#[no_mangle]
pub unsafe extern "C" fn init_randoms(seed: i32) {
    let mut j = seed.abs();

    while j >= 0x10000000 {
        j /= 2;
    }

    RANDOMS.with_borrow_mut(|randoms| {
        let mut k = 1;

        for i in 0..55 {
            let jj = k;
            k = j - k;
            j = jj;
            if k < 0 {
                k += 0x10000000;
            }
            randoms[(i * 21) % 55] = j;
        }

        new_randoms(randoms);
        new_randoms(randoms);
        new_randoms(randoms);
    });
}

#[no_mangle]
pub unsafe extern "C" fn unif_rand(x: i32) -> i32 {
    RANDOMS.with_borrow_mut(|randoms| {
        if J_RANDOM.get() == 0 {
            new_randoms(randoms);
        } else {
            J_RANDOM.set(J_RANDOM.get() - 1);
        }

        let y = take_frac(x.abs(), randoms[J_RANDOM.get()]);
        if y == x.abs() {
            0
        } else if x > 0 {
            y
        } else {
            -y
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn norm_rand() -> i32 {
    RANDOMS.with_borrow_mut(|randoms| {
        let mut x;
        let mut u;

        loop {
            loop {
                if J_RANDOM.get() == 0 {
                    new_randoms(randoms);
                } else {
                    J_RANDOM.set(J_RANDOM.get() - 1);
                }

                x = take_frac(112429, randoms[J_RANDOM.get()] - 0x08000000);

                if J_RANDOM.get() == 0 {
                    new_randoms(randoms);
                } else {
                    J_RANDOM.set(J_RANDOM.get() - 1);
                }

                u = randoms[J_RANDOM.get()];

                if x.abs() < u {
                    break;
                }
            }

            x = make_frac(x, u);
            let l = 139548960 - m_log(u);

            if ab_vs_cd(1024, l, x, x) >= 0 {
                break;
            }
        }

        x
    })
}
