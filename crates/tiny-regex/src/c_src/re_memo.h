#ifndef RE_MEMO_H
#define RE_MEMO_H

#include <stddef.h>

/* Context capturing the memoisation state at the entry point of matchpattern.
 * Declare const: the pointer value and offsets never change after init, though
 * RE_MEMO_SET writes through ctx.memo into the table pointed to. */
typedef struct {
    unsigned char* memo;
    size_t         entry_node;
    size_t         entry_offset;
    size_t         memo_stride;  /* text-dimension stride; passed in from re_matchp */
} re_memo_ctx_t;

/* Flat bit index for the entry point captured in ctx. */
#define RE_MEMO_IDX(ctx) \
    ((ctx).entry_node * (ctx).memo_stride + (ctx).entry_offset)

/* Test whether the entry point is recorded as failed.
 * ctx.memo is unsigned char* (a flat bit array). */
#define RE_MEMO_GET(ctx) \
    (((ctx).memo[RE_MEMO_IDX(ctx) / 8u] >> (RE_MEMO_IDX(ctx) % 8u)) & 1u)

/* Record the entry point as failed. */
#define RE_MEMO_SET(ctx) \
    ((ctx).memo[RE_MEMO_IDX(ctx) / 8u] |= (1u << (RE_MEMO_IDX(ctx) % 8u)))

/* Evaluate r_expr exactly once.  On failure (result == 0): restore the
 * caller's matchlength to its pre-call value and, if within the memoised
 * range, record the failure.  Then return the result.
 *
 * Intended as the sole exit point of a QUESTIONMARK/STAR/PLUS/END branch. */
#define RE_MEMO_FAIL(memo_ctx, match_ctx, pre_len, r_expr) do {              \
    int _r = (r_expr);                                                        \
    if (!_r) {                                                                \
        *(match_ctx)->matchlength = (pre_len);                                \
        if ((memo_ctx).memo && (memo_ctx).entry_offset < (memo_ctx).memo_stride) \
            RE_MEMO_SET(memo_ctx);                                            \
    }                                                                         \
    return _r;                                                                \
} while (0)

#endif /* RE_MEMO_H */
