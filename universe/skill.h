/*\
 *  Role playing, management and strategy game
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

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
