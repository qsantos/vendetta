#ifndef U_MATERIAL_H
#define U_MATERIAL_H

typedef struct kindOf_material kindOf_material_t;

#include <wchar.h>

#include "skill.h"

struct kindOf_material
{
	wchar_t* name;

	kindOf_skill_t skill;
};

void kindOf_material_init(kindOf_material_t* m);
void kindOf_material_exit(kindOf_material_t* m);

#endif
