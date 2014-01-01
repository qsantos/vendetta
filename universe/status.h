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

#ifndef U_STATUS_H
#define U_STATUS_H

typedef struct kindOf_status kindOf_status_t;

#include <wchar.h>

typedef enum
{
	ST_HEALTH,
	ST_STAMINA,
	ST_MORAL,
	ST_MANA,
	N_STATUSES,
} status_type_t;

struct kindOf_status
{
	wchar_t* name;
};

void kindOf_status_init(kindOf_status_t* s);
void kindOf_status_exit(kindOf_status_t* s);

#endif
