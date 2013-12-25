#ifndef U_SKILL_H
#define U_SKILL_H

typedef struct kindOf_skill kindOf_skill_t;

#include <wchar.h>

typedef enum
{
	SK_BUILD,
	SK_DESTROY,
	SK_CRITICAL,
	SK_TRADE,
	N_SPECIAL_SKILLS,
} specialSkill_t;

struct kindOf_skill
{
	wchar_t* name;
};

void kindOf_skill_init(kindOf_skill_t* s);
void kindOf_skill_exit(kindOf_skill_t* s);

#endif
