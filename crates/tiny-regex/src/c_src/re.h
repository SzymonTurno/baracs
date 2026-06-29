/*
 *
 * Mini regex-module inspired by Rob Pike's regex code described in:
 *
 * http://www.cs.princeton.edu/courses/archive/spr09/cos333/beautiful.html
 *
 *
 *
 * Supports:
 * ---------
 *   '.'        Dot, matches any character
 *   '^'        Start anchor, matches beginning of string
 *   '$'        End anchor, matches end of string
 *   '*'        Asterisk, match zero or more (greedy)
 *   '+'        Plus, match one or more (greedy)
 *   '?'        Question, match zero or one (greedy)
 *   '[abc]'    Character class, match if one of {'a', 'b', 'c'}
 *   '[^abc]'   Inverted class, match if NOT one of {'a', 'b', 'c'}
 *   '[a-zA-Z]' Character ranges, the character set of the ranges { a-z | A-Z }
 *   '\s'       Whitespace, \t \f \r \n \v and spaces
 *   '\S'       Non-whitespace
 *   '\w'       Alphanumeric, [a-zA-Z0-9_]
 *   '\W'       Non-alphanumeric
 *   '\d'       Digits, [0-9]
 *   '\D'       Non-digits
 *
 *
 */

#ifndef _TINY_REGEX_C
#define _TINY_REGEX_C

#include <stddef.h>

#ifdef __cplusplus
extern "C"{
#endif


/* Compiled pattern node: a type tag plus either a literal character or a
 * byte offset into the caller-supplied ccl_buf for character-class patterns.
 * Using an offset (not a pointer) keeps regex_t self-contained so the compiled
 * pattern can be freely copied or moved without pointer-fixup. */
typedef struct regex_t
{
  unsigned char  type;    /* CHAR, STAR, etc.                               */
  union
  {
    unsigned char  ch;    /*      the character itself  (CHAR nodes)        */
    size_t         ccl;   /*  OR  byte offset into ccl_buf (CHAR_CLASS /   */
                          /*      INV_CHAR_CLASS nodes)                     */
  } u;
} regex_t;

/* Typedef'd pointer to get abstract datatype. */
typedef const regex_t* re_t;


/* Compile regex string pattern into the caller-supplied node array.
 *
 * re_compiled — caller-allocated array of at least `size` regex_t nodes.
 * size        — capacity of re_compiled in nodes (must be >= 2).
 * ccl_buf     — caller-allocated character-class buffer of `ccl_size` bytes.
 * ccl_size    — capacity of ccl_buf in bytes (must be >= 2).
 *
 * Returns the number of bytes written into re_compiled on success, or 0 on
 * failure.  A return value of 0 means the pattern is invalid or a buffer is
 * too small. */
size_t re_compile(regex_t* re_compiled, size_t size,
                  unsigned char* ccl_buf, size_t ccl_size,
                  const char* pattern);


/* Find matches of the compiled pattern inside text.
 *
 * ccl_buf     — the character-class buffer supplied to re_compile.
 * memo        — caller-supplied byte array of memo_bytes bytes; re_matchp
 *               zeros it internally before each attempt.  Pass NULL / 0 to
 *               disable memoisation.
 * memo_bytes  — size of the memo array in bytes.
 * memo_stride — text-offset dimension of the memo table; memo covers offsets
 *               [0, memo_stride) per pattern node.  Ignored when memo is NULL. */
int re_matchp(re_t pattern, const unsigned char* ccl_buf,
              const char* text, int* matchlength,
              unsigned char* memo, size_t memo_bytes, size_t memo_stride);


#ifdef __cplusplus
}
#endif

#endif /* ifndef _TINY_REGEX_C */
