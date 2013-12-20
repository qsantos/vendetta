#include <stdlib.h>
#include <stdio.h>
#include <SFML/Graphics.h>

#include "world.h"
#include "draw.h"

int main(void)
{
	sfVideoMode mode = {800, 600, 32};
	sfRenderWindow* window = sfRenderWindow_create(mode, "Vendetta 0.2", sfResize | sfClose, NULL);
	if (window == NULL)
		exit(1);

	sfTexture* texture = sfTexture_createFromFile("character.png", NULL);
	if (texture == NULL)
		exit(1);

	world_t world;
	world_init(&world);

	character_t* player = &world.characters[0];

	const sfView* default_view = sfRenderWindow_getDefaultView(window);
	sfView* world_view = sfView_copy(default_view);

	sfClock* clock = sfClock_create();
	while (sfRenderWindow_isOpen(window))
	{
		float duration = sfTime_asSeconds(sfClock_getElapsedTime(clock));
		sfClock_restart(clock);

		sfEvent event;
		while (sfRenderWindow_pollEvent(window, &event))
		{
			if (event.type == sfEvtClosed)
			{
				sfRenderWindow_close(window);
			}
			else if (event.type == sfEvtKeyPressed)
			{
				if (event.key.code == sfKeyEscape)
					sfRenderWindow_close(window);
			}
			else if (event.type == sfEvtMouseButtonReleased)
			{
				sfMouseButtonEvent* e = &event.mouseButton;
				if (e->button != sfMouseLeft)
					continue;

				sfVector2i pix = {e->x, e->y};
				sfVector2f pos = sfRenderWindow_mapPixelToCoords(window, pix, world_view);
				player->go_x = pos.x;
				player->go_y = pos.y;
			}
		}

		sfBool up    = sfKeyboard_isKeyPressed(sfKeyUp);
		sfBool down  = sfKeyboard_isKeyPressed(sfKeyDown);
		sfBool left  = sfKeyboard_isKeyPressed(sfKeyLeft);
		sfBool right = sfKeyboard_isKeyPressed(sfKeyRight);

		if (up || down || left || right)
		{
			player->go_x = player->x + 100 * (right - 2*left);
			player->go_y = player->y + 100 * (down  - 2*up);
		}

		world_doRound(&world, duration);

		sfRenderWindow_clear(window, sfBlack);

		sfVector2f pos = {player->x, player->y};
		sfView_setCenter(world_view, pos);
		sfRenderWindow_setView(window, world_view);

		draw_world(window, &world);

		sfRenderWindow_setView(window, default_view);

		sfRenderWindow_display(window);
	}

	sfClock_destroy(clock);
//	sfSprite_destroy(sprite); // TODO
//	sfTexture_destroy(texture);
	sfRenderWindow_destroy(window);
	return 0;
}
