import pygame
import math
from robot import Robot
import time

# Global variable to record joint positions
joint_positions = {}

def main():
    global joint_positions  # Indicate that we're using the global variable

    robot = Robot()
    robot.initialize()

    servo_id_to_joint = {joint.servo_id: joint for joint in robot.joints}

    def prompt_for_joint():
        while True:
            user_input = input("Enter the servo ID you want to control: ").strip()
            filtered_input = ''.join(filter(str.isdigit, user_input))
            if not filtered_input:
                print("Invalid input. Please enter a numeric servo ID.")
                continue
            try:
                servo_id_input = int(filtered_input)
                if servo_id_input not in servo_id_to_joint:
                    print(f"Servo ID {servo_id_input} not found.")
                    continue
                return servo_id_to_joint[servo_id_input]
            except ValueError:
                print("Invalid input. Please enter a numeric servo ID.")

    pygame.init()
    screen = pygame.display.set_mode((400, 300))
    pygame.display.set_caption("Servo Control")

    # Get initial servo states and populate joint_positions
    robot.get_servo_states()
    for joint in robot.joints:
        joint_positions[joint.name] = joint.current_position

    joint = prompt_for_joint()
    if not joint:
        return

    print(f"Controlling joint '{joint.name}' with servo ID {joint.servo_id}")

    # Retrieve the current position from joint_positions
    current_position = joint_positions.get(joint.name, joint.current_position)
    print(f"Joint '{joint.name}' current angle is {math.degrees(current_position):.2f} degrees")

    running = True
    while running:
        time.sleep(0.01)
        for event in pygame.event.get():
            if event.type == pygame.QUIT:
                running = False

            elif event.type == pygame.KEYDOWN:
                if event.key == pygame.K_ESCAPE:
                    running = False

                elif event.key == pygame.K_q:
                    # Before switching joints, update the position of the current joint
                    joint_positions[joint.name] = current_position

                    new_joint = prompt_for_joint()
                    if new_joint:
                        joint = new_joint
                        # Retrieve the position for the new joint
                        current_position = joint_positions.get(joint.name, joint.current_position)
                        print(f"Switched to controlling joint '{joint.name}' with servo ID {joint.servo_id}")
                        print(f"Joint '{joint.name}' current angle is {math.degrees(current_position):.2f} degrees")

                elif event.key == pygame.K_UP:
                    # Increase joint angle by 10 degrees
                    current_position += math.radians(10)
                    robot.set_servo_positions_by_name({joint.name: current_position})
                    joint_positions[joint.name] = current_position  # Update the position in joint_positions
                    print(f"Joint '{joint.name}' angle increased to {math.degrees(current_position):.2f} degrees")

                elif event.key == pygame.K_DOWN:
                    # Decrease joint angle by 10 degrees
                    current_position -= math.radians(10)
                    robot.set_servo_positions_by_name({joint.name: current_position})
                    joint_positions[joint.name] = current_position  # Update the position in joint_positions
                    print(f"Joint '{joint.name}' angle decreased to {math.degrees(current_position):.2f} degrees")

        try:
            pass
        except KeyboardInterrupt:
            print("\nCtrl+C detected, shutting down gracefully...")
            running = False

    try:
        robot.disable_motors()
        print("Motors disabled")
    except Exception as e:
        print(f"Error disabling motors: {e}")
    pygame.quit()

if __name__ == "__main__":
    main()
