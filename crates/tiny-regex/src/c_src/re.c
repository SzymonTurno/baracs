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



#include "re.h"
#include "re_memo.h"
#include <ctype.h>

#ifndef RE_DOT_MATCHES_NEWLINE
/* Define to 1 if '.' should match '\r' and '\n'. */
#define RE_DOT_MATCHES_NEWLINE 0
#endif
#include <string.h>

enum { UNUSED, DOT, BEGIN, END, QUESTIONMARK, STAR, PLUS, CHAR, CHAR_CLASS, INV_CHAR_CLASS, DIGIT, NOT_DIGIT, ALPHA, NOT_ALPHA, WHITESPACE, NOT_WHITESPACE, /* BRANCH */ };


/* Context shared across all match functions for a single re_matchp call. */
typedef struct {
  const regex_t*       pat_base;
  const unsigned char* text_base;
  const unsigned char* ccl_buf;    /* character-class buffer for offset dereference */
  int*                 matchlength;
  unsigned char*       memo;
  size_t               memo_stride; /* text-dimension stride of the memo bit array */
} re_match_ctx_t;



/* Private function declarations: */
static int matchpattern(const regex_t* pattern, const unsigned char* text, re_match_ctx_t* ctx);
static int matchcharclass(unsigned char c, const unsigned char* str);
static int matchstar(regex_t p, const regex_t* pattern, const unsigned char* text, re_match_ctx_t* ctx);
static int matchplus(regex_t p, const regex_t* pattern, const unsigned char* text, re_match_ctx_t* ctx);
static int matchquestion(regex_t p, const regex_t* pattern, const unsigned char* text, re_match_ctx_t* ctx);
static int matchone(regex_t p, unsigned char c, const unsigned char* ccl_buf);
static int matchdigit(unsigned char c);
static int matchalpha(unsigned char c);
static int matchwhitespace(unsigned char c);
static int matchmetachar(unsigned char c, const unsigned char* str);
static int matchrange(unsigned char c, const unsigned char* str);
static int matchdot(unsigned char c);



/* Pass memo_bytes==0 to disable memoisation; re_matchp zeros the table itself. */
int re_matchp(re_t pattern, const unsigned char* ccl_buf,
              const char* text, int* matchlength,
              unsigned char* memo, size_t memo_bytes, size_t memo_stride)
{
  /* Cast once at the public boundary so all internal helpers see unsigned char,
   * eliminating sign-extension issues with bytes > 127 (e.g. UTF-8 sequences)
   * without scattering casts throughout the match functions. */
  const unsigned char* utext = (const unsigned char*)text;
  *matchlength = 0;
  if (pattern != 0)
  {
    /* Treat a zero-size memo as disabled so callers may always pass a pointer
     * (e.g. a zero-length array) without special-casing NULL at the call site. */
    if (!memo_bytes) memo = NULL;
    re_match_ctx_t ctx = { pattern, utext, ccl_buf, matchlength, memo, memo_stride };
    /* Zero the memo table at entry so anchored patterns (BEGIN) are covered
     * without depending on the caller.  The outer loop re-zeroes before every
     * non-anchored attempt, so the entry zero is redundant there. */
    if (memo) memset(memo, 0, memo_bytes);
    if (pattern[0].type == BEGIN)
    {
      return ((matchpattern(&pattern[1], utext, &ctx)) ? 0 : -1);
    }
    else
    {
      int idx = -1;

      do
      {
        idx += 1;
        /* Per-attempt reset: text_base makes memo offsets relative to this
         * starting position so any-length input gets full coverage. */
        ctx.text_base = utext;
        if (memo) memset(memo, 0, memo_bytes);

        if (matchpattern(pattern, utext, &ctx))
        {
          return idx;
        }
      }
      while (*utext++ != '\0');
    }
  }
  return -1;
}

size_t re_compile(regex_t* re_compiled, size_t size,
                  unsigned char* ccl_buf, size_t ccl_size,
                  const char* pattern)
{
  /* ccl_size determines the size of buffer for chars in all char-classes in the expression. */
  size_t ccl_bufidx = 1;

  char c;        /* current char in pattern   */
  size_t i = 0;  /* index into pattern        */
  size_t j = 0;  /* index into re_compiled    */

  while (pattern[i] != '\0' && (j+1 < size))
  {
    c = pattern[i];

    switch (c)
    {
      /* Meta-characters: */
      case '^': {    re_compiled[j].type = BEGIN;           } break;
      case '$': {    re_compiled[j].type = END;             } break;
      case '.': {    re_compiled[j].type = DOT;             } break;
      case '*': {    re_compiled[j].type = STAR;            } break;
      case '+': {    re_compiled[j].type = PLUS;            } break;
      case '?': {    re_compiled[j].type = QUESTIONMARK;    } break;
/*    case '|': {    re_compiled[j].type = BRANCH;          } break; <-- not working properly */

      /* Escaped character-classes (\s \w ...): */
      case '\\':
      {
        if (pattern[i+1] != '\0')
        {
          /* Skip the escape-char '\\' */
          i += 1;
          /* ... and check the next */
          switch (pattern[i])
          {
            /* Meta-character: */
            case 'd': {    re_compiled[j].type = DIGIT;            } break;
            case 'D': {    re_compiled[j].type = NOT_DIGIT;        } break;
            case 'w': {    re_compiled[j].type = ALPHA;            } break;
            case 'W': {    re_compiled[j].type = NOT_ALPHA;        } break;
            case 's': {    re_compiled[j].type = WHITESPACE;       } break;
            case 'S': {    re_compiled[j].type = NOT_WHITESPACE;   } break;

            /* Escaped character, e.g. '.' or '$' */
            default:
            {
              re_compiled[j].type = CHAR;
              re_compiled[j].u.ch = pattern[i];
            } break;
          }
        }
        /* '\\' as last char in pattern -> invalid regular expression. */
        else
        {
          return 0;
        }
      } break;

      /* Character class: */
      case '[':
      {
        /* Remember where the char-buffer starts. */
        size_t buf_begin = ccl_bufidx;

        /* Look-ahead to determine if negated */
        if (pattern[i+1] == '^')
        {
          re_compiled[j].type = INV_CHAR_CLASS;
          i += 1; /* Increment i to avoid including '^' in the char-buffer */
          if (pattern[i+1] == 0) /* incomplete pattern, missing non-zero char after '^' */
          {
            return 0;
          }
        }
        else
        {
          re_compiled[j].type = CHAR_CLASS;
        }

        /* Copy characters inside [..] to buffer */
        while (    (pattern[++i] != ']')
                && (pattern[i]   != '\0')) /* Missing ] */
        {
          if (pattern[i] == '\\')
          {
            if (ccl_bufidx + 1 >= ccl_size)
            {
              return 0;
            }
            if (pattern[i+1] == 0) /* incomplete pattern, missing non-zero char after '\\' */
            {
              return 0;
            }
            ccl_buf[ccl_bufidx++] = pattern[i++];
          }
          else if (ccl_bufidx >= ccl_size)
          {
              return 0;
          }
          ccl_buf[ccl_bufidx++] = pattern[i];
        }
        if (ccl_bufidx >= ccl_size)
        {
            /* Catches cases such as [00000000000000000000000000000000000000][ */
            return 0;
        }
        /* Null-terminate string end */
        ccl_buf[ccl_bufidx++] = 0;
        /* Store byte offset into ccl_buf rather than a raw pointer, so the
         * compiled pattern can be safely moved or copied. */
        re_compiled[j].u.ccl = buf_begin;
      } break;

      /* Other characters: */
      default:
      {
        re_compiled[j].type = CHAR;
        re_compiled[j].u.ch = c;
      } break;
    }
    /* no buffer-out-of-bounds access on invalid patterns - see https://github.com/kokke/tiny-regex-c/commit/1a279e04014b70b0695fba559a7c05d55e6ee90b */
    if (pattern[i] == 0)
    {
      return 0;
    }

    i += 1;
    j += 1;
  }
  /* Pattern too long to fit — truncation would silently change semantics. */
  if (pattern[i] != '\0')
  {
    return 0;
  }

  /* 'UNUSED' is a sentinel used to indicate end-of-pattern */
  re_compiled[j].type = UNUSED;

  return (j + 1) * sizeof(re_compiled[0]);
}



/* Private functions: */
static int matchdigit(unsigned char c)
{
  return isdigit(c);
}
static int matchalpha(unsigned char c)
{
  return isalpha(c);
}
static int matchwhitespace(unsigned char c)
{
  return isspace(c);
}
static int matchalphanum(unsigned char c)
{
  return ((c == '_') || matchalpha(c) || matchdigit(c));
}
static int matchrange(unsigned char c, const unsigned char* str)
{
  return (    (c != '-')
           && (str[0] != '\0')
           && (str[0] != '-')
           && (str[1] == '-')
           && (str[2] != '\0')
           && (    (c >= str[0])
                && (c <= str[2])));
}
static int matchdot(unsigned char c)
{
#if RE_DOT_MATCHES_NEWLINE
  (void)c;
  return 1;
#else
  return c != '\n' && c != '\r';
#endif
}
static int matchmetachar(unsigned char c, const unsigned char* str)
{
  switch (str[0])
  {
    case 'd': return  matchdigit(c);
    case 'D': return !matchdigit(c);
    case 'w': return  matchalphanum(c);
    case 'W': return !matchalphanum(c);
    case 's': return  matchwhitespace(c);
    case 'S': return !matchwhitespace(c);
    default:  return (c == str[0]);
  }
}

static int matchcharclass(unsigned char c, const unsigned char* str)
{
  do
  {
    if (matchrange(c, str))
    {
      return 1;
    }
    else if (str[0] == '\\')
    {
      /* Escape-char: increment str-ptr and match on next char */
      str += 1;
      if (matchmetachar(c, str))
      {
        return 1;
      }
    }
    else if (c == str[0])
    {
      if (c == '-')
      {
        /* Detect a literal '-' by checking whether it sits at the very start
         * or very end of the class content.  str[-1] is safe because
         * ccl_bufidx starts at 1 in re_compile, so str always points to
         * position >= 1 inside ccl_buf, and str[-1] == ccl_buf[0].
         * INVARIANT: ccl_buf[0] must be '\0' — the caller (Rust compile())
         * zero-initialises the entire buffer before every re_compile call. */
        return ((str[-1] == '\0') || (str[1] == '\0'));
      }
      else
      {
        return 1;
      }
    }
  }
  while (*str++ != '\0');

  return 0;
}

static int matchone(regex_t p, unsigned char c, const unsigned char* ccl_buf)
{
  switch (p.type)
  {
    case DOT:            return matchdot(c);
    case CHAR_CLASS:     return  matchcharclass(c, ccl_buf + p.u.ccl);
    case INV_CHAR_CLASS: return !matchcharclass(c, ccl_buf + p.u.ccl);
    case DIGIT:          return  matchdigit(c);
    case NOT_DIGIT:      return !matchdigit(c);
    case ALPHA:          return  matchalphanum(c);
    case NOT_ALPHA:      return !matchalphanum(c);
    case WHITESPACE:     return  matchwhitespace(c);
    case NOT_WHITESPACE: return !matchwhitespace(c);
    case CHAR:           return  (p.u.ch == c);
    default:             return 0;
  }
}

static int matchstar(regex_t p, const regex_t* pattern, const unsigned char* text, re_match_ctx_t* ctx)
{
  int prelen = *ctx->matchlength;
  const unsigned char* prepoint = text;
  while ((text[0] != '\0') && matchone(p, *text, ctx->ccl_buf))
  {
    text++;
    (*ctx->matchlength)++;
  }
  int n = (int)(text - prepoint);
  do {
    if (matchpattern(pattern, &prepoint[n], ctx))
      return 1;
    (*ctx->matchlength)--;
  } while (n-- > 0);

  *ctx->matchlength = prelen;
  return 0;
}

static int matchplus(regex_t p, const regex_t* pattern, const unsigned char* text, re_match_ctx_t* ctx)
{
  const unsigned char* prepoint = text;
  while ((text[0] != '\0') && matchone(p, *text, ctx->ccl_buf))
  {
    text++;
    (*ctx->matchlength)++;
  }
  /* Backtrack from the greedy end towards prepoint+1 (strictly greater, so
   * text-- never forms a pointer before prepoint — no UB, unlike matchstar's
   * old >= condition).  On failure the loop leaves *matchlength equal to its
   * value on entry: each (*matchlength)-- undoes one greedy increment, and
   * the inner matchpattern call restores its own sub-increments on failure. */
  while (text > prepoint)
  {
    if (matchpattern(pattern, text--, ctx))
      return 1;
    (*ctx->matchlength)--;
  }

  return 0;
}

static int matchquestion(regex_t p, const regex_t* pattern, const unsigned char* text, re_match_ctx_t* ctx)
{
  if (*text && matchone(p, *text, ctx->ccl_buf))
  {
    (*ctx->matchlength)++;
    if (matchpattern(pattern, text + 1, ctx))
      return 1;
    (*ctx->matchlength)--;
  }
  return matchpattern(pattern, text, ctx);
}


#if 0

/* Recursive matching */
static int matchpattern(const regex_t* pattern, const unsigned char* text, re_match_ctx_t* ctx)
{
  int pre = *ctx->matchlength;
  if ((pattern[0].type == UNUSED) || (pattern[1].type == QUESTIONMARK))
  {
    return matchquestion(pattern[1], &pattern[2], text, ctx);
  }
  else if (pattern[1].type == STAR)
  {
    return matchstar(pattern[0], &pattern[2], text, ctx);
  }
  else if (pattern[1].type == PLUS)
  {
    return matchplus(pattern[0], &pattern[2], text, ctx);
  }
  else if ((pattern[0].type == END) && pattern[1].type == UNUSED)
  {
    return text[0] == '\0';
  }
  else if ((text[0] != '\0') && matchone(pattern[0], text[0], ctx->ccl_buf))
  {
    (*ctx->matchlength)++;
    return matchpattern(&pattern[1], text+1, ctx);
  }
  else
  {
    *ctx->matchlength = pre;
    return 0;
  }
}

#else

/* Iterative matching */
static int matchpattern(const regex_t* pattern, const unsigned char* text, re_match_ctx_t* ctx)
{
  const re_memo_ctx_t memo_ctx = {
    ctx->memo,
    (size_t)(pattern - ctx->pat_base),
    (size_t)(text    - ctx->text_base),
    ctx->memo_stride,
  };

  /* Memoization: if this (pattern node, text offset) state has already been
   * proven to fail, skip it immediately. */
  if (memo_ctx.memo
      && memo_ctx.entry_offset < memo_ctx.memo_stride
      && RE_MEMO_GET(memo_ctx))
    return 0;

  int pre = *ctx->matchlength;

  do
  {
    /* Handle UNUSED separately: a full-length pattern puts UNUSED at the last
     * slot of re_compiled.  Forming &pattern[2] from that position would be
     * two past the end — UB in C11 even if matchquestion never dereferences
     * it.  Return 1 directly (the end-of-pattern success that
     * matchquestion(UNUSED,...) would give). */
    if (pattern[0].type == UNUSED)
    {
      return 1;
    }
    else if (pattern[1].type == QUESTIONMARK)
    {
      RE_MEMO_FAIL(memo_ctx, ctx, pre, matchquestion(pattern[0], &pattern[2], text, ctx));
    }
    else if (pattern[1].type == STAR)
    {
      RE_MEMO_FAIL(memo_ctx, ctx, pre, matchstar(pattern[0], &pattern[2], text, ctx));
    }
    else if (pattern[1].type == PLUS)
    {
      RE_MEMO_FAIL(memo_ctx, ctx, pre, matchplus(pattern[0], &pattern[2], text, ctx));
    }
    else if ((pattern[0].type == END) && pattern[1].type == UNUSED)
    {
      RE_MEMO_FAIL(memo_ctx, ctx, pre, (text[0] == '\0'));
    }
  (*ctx->matchlength)++;
  }
  while ((text[0] != '\0') && matchone(*pattern++, *text++, ctx->ccl_buf));

  *ctx->matchlength = pre;
  if (memo_ctx.memo && memo_ctx.entry_offset < memo_ctx.memo_stride)
    RE_MEMO_SET(memo_ctx);
  return 0;
}

#endif
