#include "skill.h"

#include <stdlib.h>

void kindOf_skill_init(kindOf_skill_t* s)
{
	s->name = NULL;
}

void kindOf_skill_exit(kindOf_skill_t* s)
{
	free(s->name);
}
